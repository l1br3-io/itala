# ADR-0003 — `async-trait` for the object-safe `DynVfs` shadow

- **Status:** Accepted
- **Date:** 2026-07-11

## Context

The VFS is a port (hexagonal architecture): backends implement it, contexts and the plugin host consume it. Consumers that hold "some VFS backend" at runtime — the plugin host's `PluginContext`, future contexts — need a **trait object**: `Box<dyn DynVfs>` / `Arc<dyn DynVfs>`.

Native `async fn` in traits (stable in Rust 1.93) is object-unsafe: the returned future is an opaque `impl Future` with no name, so no vtable can be built for `dyn Trait`. For the static-dispatch path (a concrete backend used directly) native async is ideal — zero overhead, no allocation. For the runtime-dispatch path, something has to give.

The `async-trait` crate rewrites `async fn` methods into `fn … -> Pin<Box<dyn Future + Send>>`, which *is* object-safe at the cost of one heap allocation per call. For VFS I/O (which is already async and I/O-bound), that allocation is negligible.

## Decision

The VFS port is modeled as **two traits**:

- `Vfs` — native `async fn` in trait, for monomorphized / static-dispatch call sites. *Not* object-safe.
- `DynVfs` — the object-safe shadow built with `#[async_trait]`, used as `Box<dyn DynVfs>` / `Arc<dyn DynVfs>` by the plugin host and contexts.

A blanket adapter `IntoDyn<V: Vfs>` + `into_dyn(vfs)` bridges any concrete `Vfs` into a `Box<dyn DynVfs>` for runtime dispatch. Method signatures are identical across the two traits.

## Consequences

- **Positive:** Static-dispatch callers pay nothing; runtime-dispatch callers get a working, object-safe trait object.
- **Positive:** Backends implement `Vfs` once; the host gets `DynVfs` for free via `into_dyn`.
- **Negative:** A small per-call allocation on the `DynVfs` path. For VFS I/O this is unmeasurable relative to the I/O itself, but it would be the wrong choice for the keystroke hot path (which stays off the VFS entirely and uses direct trait calls).
- **Neutral:** When the language gains object-safe `async fn in trait` (eventually), the `DynVfs` shadow can be removed and `dyn Vfs` used directly — but there is no urgency.
