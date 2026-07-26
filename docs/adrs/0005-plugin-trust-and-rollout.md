# ADR-0005 — Trusted native components now; sandboxed WASM extensions at M5

- **Status:** Accepted
- **Date:** 2026-07-19
- **Supersedes:** [ADR-0002](./0002-rust-native-plugins-m0.md)

## Context

M0 uses compile-time-registered native Rust plugins to prove manifests, lifecycle management, capability checks, audit records, and failure handling without also introducing a WASM component boundary. ADR-0002 described those capability checks as a sandbox that limited a native plugin's blast radius.

That description overstated the security boundary. Native code executes in the host process with the process's ambient authority. It can bypass `PluginContext`, access the filesystem or network directly, abort the process, corrupt shared state through unsafe dependencies, or otherwise ignore host policy. Catching a lifecycle panic improves resilience to ordinary bugs but does not isolate malicious or severely faulty native code.

The earlier vision also listed every functional area and several runtimes as public plugin surfaces from the start. Stabilizing editors, storage, sync, search, AI, UI extensions, native loading, WASM, and scripting together would front-load a large compatibility and security burden before those contracts have real implementations.

## Decision

1. **M0–M4 native plugins are trusted first-party components.** They remain useful for proving lifecycle and service contracts, but capability checks are policy and audit controls, not a security sandbox.
2. **M5 introduces WASM components as the default third-party runtime.** Third-party modules receive no ambient filesystem, network, process, or host access. All privileged operations cross capability-checked host imports.
3. **The initial public surfaces are narrow:** tools, commands, exporters, and renderers. Maintained first-party plugins prove each contract before third-party stability is promised.
4. **Deeper replacement surfaces are future work.** Alternative editors, themes, storage, sync, search, and AI providers become public only after their first-party ports and product behavior are proven.
5. **The plugin API is experimental before 1.0.** A compatibility and deprecation policy must be written before declaring it stable. Indefinite coexistence of every historical ABI is not assumed.
6. **Distribution begins simply.** Local package installation and a curated signed index ship before ratings, reviews, popularity ranking, or automatic remote disabling. Signatures establish package identity and integrity. A future registry may publish revocation advisories, but automatic action requires a separate consent and governance decision.

## Consequences

- **Positive:** Security claims match the actual runtime boundary.
- **Positive:** M0's completed work remains valid as a trusted in-process extension mechanism and test bed.
- **Positive:** M5 can focus Rust learning on one meaningful WASM component boundary instead of several runtimes and every possible plugin type.
- **Positive:** First-party implementations can change while their contracts are still being discovered.
- **Negative:** Native third-party plugins are not supported by the initial ecosystem, including performance-sensitive extensions that cannot fit the WASM boundary.
- **Negative:** Editor, storage, sync, search, and AI replacement arrive later than originally envisioned.
- **Neutral:** Capability names and audit records remain useful for native components even though they are not a security boundary there.
