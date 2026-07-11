# ADR-0002 — Rust-native plugins for M0, WASM deferred to M5

- **Status:** Accepted
- **Date:** 2026-07-11

## Context

The architecture targets WebAssembly (`wasmtime`) for third-party plugins so they run in a memory-safe, capability-gated sandbox with a versioned WIT/ABI boundary. But `wasmtime` is a massive dependency and the host/guest WIT boundary is a project unto itself. Shipping it for M0 — a milestone whose job is to stand up the kernel, VFS, and plugin *host* machinery — would couple two large risks: getting the host lifecycle/capability story right, and getting the WASM sandbox right.

At the same time, the capability model (plugins declare capabilities; the host enforces every privileged call) is the actual safety boundary that matters for M0: it limits what a plugin can *do* regardless of the runtime it executes in. WASM adds defense-in-depth (memory safety) on top of that, but it is not the thing being proven in M0.

## Decision

**M0 ships Rust-native plugins.** Plugins implement traits from `itala-plugin-api` and are registered at startup via a host factory (`PluginHost::register_factory`); no `dlopen` of `cdylib` plugins in M0. The capability system still sandboxes: every privileged call goes through the host's `PluginContext`, which checks the `CapabilityRegistry` and returns `Error::CapabilityDenied` on a missing grant.

WASM sandboxing is deferred to **M5 (Extend)**, when the plugin ecosystem milestone arrives. The `Plugin`/`PluginContext` trait is designed to be WASM-compatible from the start — host-only types stay out of the contract, and `Send` bounds are explicit — so the eventual WASM runtime is an additive change to the host, not a rewrite of the plugin contract.

## Consequences

- **Positive:** M0 stays focused and small; the host lifecycle, crash isolation (`tokio::task` per lifecycle call), capability enforcement, and audit trail are all proven without the WASM dependency surface.
- **Positive:** The plugin contract is WASM-ready by construction, so the M5 addition is a host-internal runtime swap.
- **Negative:** M0 plugins are not memory-safely sandboxed — a native plugin can, in principle, violate memory safety. The capability system limits the *blast radius* of what a compromised plugin can do through the host, but cannot prevent a native plugin from misbehaving outside the contract.
- **Neutral:** `cdylib` dynamic loading is also deferred to M5; M0 plugins are compile-time-registered, which is sufficient for M0 (no user-facing plugin install surface yet).
