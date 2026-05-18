# Rust Unsafe & FFI

> Carve-out doctrine for the few crates that lift `#![forbid(unsafe_code)]`. The default lives in `rules/rust/security.md`; this file is what you read when you're building (or reviewing) one of those crates.

## Where unsafe is permitted in Itala

By default every crate root has `#![forbid(unsafe_code)]`. The following crates may replace that with `#![deny(unsafe_op_in_unsafe_fn)]` and use `unsafe`:

- `itala-plugin-host` — wasmtime resource handles cross the host/guest boundary as raw pointers in places.
- `apps/desktop/src-tauri` and `apps/mobile/src-tauri` — JNI / Swift bridges for mobile shells.
- A future crypto hot-path crate, if and only if benchmarks justify it (none today).

Anywhere else, `unsafe` is a code review block.

## Powers `unsafe` grants

1. **Dereference raw pointers** (`*const T`, `*mut T`).
2. **Call `unsafe fn`** (FFI, unchecked stdlib methods like `String::from_utf8_unchecked`).
3. **Read or write through a mutable static**.
4. **Implement `unsafe trait`** (most often `Send`/`Sync` for a type holding raw pointers).
5. **Access fields of a `union`**.

`unsafe fn` says "the *caller* must uphold an invariant." `unsafe { … }` says "the *programmer* asserts the invariant holds here." They're separate concepts; keep them separate.

## Validity invariants you must uphold

The compiler will assume these hold and miscompile silently if they don't:

- References (`&T`, `&mut T`) are non-null, aligned, and point to a valid `T`. No aliasing between `&mut` and any other reference to the same place.
- `bool` is `0` or `1`. Any other byte is UB.
- `char` is a valid Unicode scalar value (not a surrogate, not above `0x10FFFF`).
- Enum discriminants land on a declared variant. `mem::transmute::<u8, Direction>(99)` is UB.
- `MaybeUninit<T>` is the only legal way to hold a `T`-shaped place that isn't initialized. `assume_init` asserts initialization happened; lying is UB.
- `UnsafeCell<T>` is the **only** legal way to mutate through a `&T`. Built on top by `Cell` / `RefCell` / `Mutex`.

## The `// SAFETY:` comment is mandatory

Every `unsafe { … }` block carries a `// SAFETY:` comment immediately above it stating the invariant being upheld and *why it holds here*. PR review rejects new `unsafe` blocks without one. Every `unsafe fn` carries a `# Safety` rustdoc section listing the caller's obligations.

```rust
/// Convert a wasmtime-owned byte buffer into a borrowed slice.
///
/// # Safety
/// `ptr` must be non-null and point to `len` initialized bytes that
/// remain valid for `'a`. The buffer must not be freed or aliased
/// mutably for the duration of `'a`. The only legitimate caller is
/// `wasmtime_alloc_buffer`'s consumer side.
pub unsafe fn wasm_buf_as_slice<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    // SAFETY: contract above; `len` came from the same allocation as `ptr`.
    unsafe { core::slice::from_raw_parts(ptr, len) }
}
```

## Panic safety

A panic inside an `unsafe` block can leave data structures observably broken because unwinding runs `Drop` impls on partially constructed values. The two patterns that bite:

1. **`Vec` length / capacity skew.** `set_len(new)` before initializing — if a panic happens between, drop reads uninitialized memory. Initialize first; bump length last.
2. **Foreign callbacks.** A Rust panic unwinding into a C frame is UB on most ABIs. Wrap every `extern "C" fn` callback body in `std::panic::catch_unwind` and translate the panic to an error code on the FFI boundary.

Use **drop guards** for "must run regardless" cleanup — a small struct whose `Drop` impl performs the cleanup. Bind it to `let _guard = …` (not `let _ = …`, which drops immediately).

## Pointer aliasing — raw pointers are not a free pass

Rust's aliasing model (Stacked Borrows / Tree Borrows) tracks every borrow, including those that started life as a raw pointer cast. Forming two `&mut` to the same place — even via `*mut T` — is UB. If you need shared mutation through raw pointers, the path is `UnsafeCell` and `&` borrows; never two `&mut`s.

When in doubt, run `cargo +nightly miri test` on the crate. Miri catches aliasing bugs that pass Clippy and `cargo test`.

## Drop check & `PhantomData`

If you store a `*const T` and the `T` is logically owned by your type, add `PhantomData<T>` so the drop checker treats your type as owning a `T`. Without it, the borrow checker may permit moves that should be illegal:

```rust
pub struct Owner<T> {
    ptr: *mut T,
    _marker: PhantomData<T>,   // tells dropck "I own a T"
}
```

`#[may_dangle]` on `unsafe impl Drop` is the inverse — promising you *won't* touch the generic during drop. Reach for it only when you understand drop check fully; otherwise omit it and let the conservative default win.

## FFI patterns

### Calling convention and ABI

- Default `extern "C"` for almost everything.
- `extern "C-unwind"` only when panics must traverse the boundary in either direction (rare; usually a bug).
- `#[no_mangle] pub extern "C" fn …` to export.

### Type compatibility

- Use `std::os::raw::{c_int, c_char, c_void}` for primitive C types — never `i32` directly, since on niche platforms it differs.
- `#[repr(C)]` on every struct or enum that crosses the boundary. Field order is then guaranteed.
- Strings: `std::ffi::CStr` (borrowed, NUL-terminated) and `CString` (owned). Never pass `&str` directly — it has a length, not a NUL.
- Slices: pass `(ptr, len)` separately. There is no portable C equivalent of `&[T]`.
- Opaque handles: `pub struct Handle { _private: [u8; 0] }` with `#[non_exhaustive]`. Distinct types per concept; don't reuse `*mut c_void`.
- Nullable function pointers: `Option<extern "C" fn(…)>`. The `Option` niche optimization makes this zero-cost.

### Memory ownership

The allocator that allocated the memory must free it. Patterns:

- **Caller-allocated:** the C side passes a buffer Rust fills. Simplest; no cross-boundary frees.
- **Library-allocated:** Rust exposes a `make_x` and a `free_x` that the C side must pair. Document the pairing in the header.

Never `free()` a Rust `Vec`'s buffer from C, and never `Box::from_raw` something allocated by `malloc`. Both are UB.

### Safe-wrapper crate split

Each FFI integration is two crates:

- `xxx-sys` — raw `extern` blocks, `#[repr(C)]` types, generated by `bindgen` from a vendored or upstream header. No safety claims. No public API beyond the raw bindings.
- `xxx` — safe wrappers: lifetimes connecting handles to their owning context (`Device<'_>` borrows `Context`), explicit `Send`/`Sync` claims (only when the C library documents thread safety), `Drop` impls calling the matching `free_*`.

`bindgen` runs in `build.rs` for `-sys` crates; `cbindgen` runs in `build.rs` only when we *export* a C header (rare for Itala — only the desktop shell may need this for OS integration).

## Test mandate for any crate that lifts `forbid(unsafe_code)`

If a crate uses `unsafe`, its CI must include:

- `cargo +nightly miri test` — catches UB, aliasing violations, uninitialized reads.
- AddressSanitizer and ThreadSanitizer runs on Linux: `RUSTFLAGS="-Zsanitizer=address" cargo test` and `…=thread`.
- `loom` tests for any concurrent unsafe (atomics, lock-free queues). Loom exhaustively explores thread interleavings for small cases.
- A property-test (`proptest`) that exercises the safe wrapper across realistic inputs to find the cases hand-written tests miss.

See `rules/rust/testing.md` for general test patterns; the additions above are what `unsafe` specifically demands. Without these, the safe wrapper is a *claim*, not a *guarantee*.
