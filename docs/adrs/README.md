# Architecture Decision Records (ADRs)

An **ADR** is a short, immutable document that captures *one* significant architectural decision: the context, the options considered, the choice, and the consequences. ADRs explain **why** Itala is built the way it is — the part that source code and the [architecture overview](../ARCHITECTURE.md) cannot convey on their own.

We use ADRs for decisions that are **expensive to reverse** (data formats, identity schemes, concurrency model, extension boundaries) and for choices that future contributors will reasonably question ("why CRDTs?", "why WASM plugins?").

## When to write an ADR

- You choose a foundational technology, format, or pattern (e.g., CRDT library, ID scheme).
- You define a public, hard-to-change contract (plugin ABI, on-disk vault layout, event schema).
- You reject a popular alternative in favor of a less obvious one.
- A decision spans multiple crates/contexts and needs a shared rationale.

You do **not** need an ADR for local refactors, dependency bumps, or anything fully reversible.

## Format

Each ADR follows the [Michael Nygard template](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) (see [0001](0001-record-architecture-decisions.md)):

- **Title** — `NNNN-short-slug.md`, zero-padded sequence.
- **Status** — Proposed, Accepted, Deprecated, Superseded by `NNNN`.
- **Context** — the forces at play, constraints, and alternatives considered.
- **Decision** — the choice and the direct justification.
- **Consequences** — positive, negative, and neutral effects; follow-ups.

ADRs are append-only. To change an accepted decision, write a new ADR that **supersedes** the old one and update the prior record's status — do not rewrite history.

## Index

| # | Decision | Status |
|---|----------|--------|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-rust-native-plugins-m0.md) | Rust-native plugins for M0, WASM deferred to M5 | Superseded by 0005 |
| [0003](0003-async-trait-dynvfs.md) | `async-trait` for the object-safe `DynVfs` shadow | Accepted |
| [0004](0004-derived-state-storage-strategy.md) | Per-domain ports for derived-state storage, with per-vault backend selection | Accepted |
| [0005](0005-plugin-trust-and-rollout.md) | Trusted native components now; sandboxed WASM extensions at M5 | Accepted |
| [0006](0006-markdown-canonical-source.md) | Markdown is the canonical note representation | Accepted |
| [0007](0007-best-effort-event-delivery.md) | Bounded best-effort events with explicit reconciliation | Accepted |
| [0008](0008-pure-rust-platforms-and-mobile-ffi.md) | Pure Rust everywhere; native mobile shells consume the core via UniFFI | Accepted |

## Candidate ADRs (to be written as decisions crystallize)

- ULID as the canonical note identifier.
- CRDT selection as an optional synchronization/collaboration substrate.
- Hexagonal architecture (ports & adapters) with a strict dependency direction.
- BLAKE3 as the content checksum.
