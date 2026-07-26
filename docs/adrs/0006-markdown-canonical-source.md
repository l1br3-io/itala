# ADR-0006 — Markdown is the canonical note representation

- **Status:** Accepted
- **Date:** 2026-07-19

## Context

Itala promises plain Markdown ownership and interoperability with external tools. Earlier architecture text simultaneously described a CRDT body as the source of truth and Markdown as its projection. That creates two competing authorities and makes externally edited files, recovery, migration, and operation without Itala ambiguous.

CRDTs may later be valuable for offline multi-device synchronization or real-time collaboration, but neither feature is part of the launch core. Selecting a CRDT as the permanent note model during M2 would couple the editor and durable format to a future feature before its requirements are known.

## Decision

1. A note's persisted `.md` file is its canonical representation.
2. YAML frontmatter is authoritative for persisted metadata; the Markdown body is authoritative for prose.
3. Parsed syntax trees, render trees, editing transactions, indexes, caches, and recovery journals are subordinate representations. Accepted edits must project back to Markdown.
4. External Markdown edits are first-class changes. A running editor must detect and reconcile them rather than silently overwrite them from stale in-memory state.
5. CRDTs are deferred until synchronization or collaboration work supplies concrete requirements. If adopted, CRDT state remains subordinate: accepted merged content projects back to canonical Markdown, and the vault remains usable without the CRDT engine.
6. `.itala/` contains both rebuildable derived state and durable application state. Search and graph indexes are rebuildable; configuration, capability grants, plugin records, recovery journals, and local history are not. History stores prior canonical content and must not be discarded as a cache.

## Consequences

- **Positive:** A vault remains directly readable, editable, versionable, and recoverable with ordinary filesystem tools.
- **Positive:** M2 can focus on Markdown parsing and editing semantics without prematurely committing to a synchronization algorithm.
- **Positive:** Index corruption has a clear recovery path without implying that all application metadata is disposable.
- **Negative:** Future CRDT integration must preserve a clean projection to Markdown and define conflicts with external edits.
- **Negative:** Some collaborative data, such as operation histories or presence, cannot be represented in Markdown and will remain auxiliary state.
- **Neutral:** Deterministic projection does not require preserving every intermediate editor data structure; the persisted Markdown contract is what remains authoritative.
