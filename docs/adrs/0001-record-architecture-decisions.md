# ADR-0001 — Record architecture decisions

- **Status:** Accepted
- **Date:** 2026-07-10

## Context

Itala is a greenfield project with an ambitious, plugin-driven architecture. Many of its foundational choices — the ID scheme, the CRDT substrate, the plugin sandbox, the on-disk format, the cross-context communication model — are expensive to reverse once shipped and once third-party plugins depend on them.

Code and the [architecture overview](../ARCHITECTURE.md) describe *what* the system is and *how* it is put together, but they do not reliably preserve *why* a particular path was chosen, what alternatives were rejected, or what trade-offs were accepted. Without that record, decisions get re-litigated, contributors reinvent rationale, and silent drift occurs.

## Decision

We will keep Architecture Decision Records (ADRs) in [`docs/adrs/`](./), one file per significant decision, using the Michael Nygard template (Context, Decision, Consequences). ADRs are numbered, append-only, and dated. To change an accepted decision we supersede it with a new ADR rather than editing the original.

## Consequences

- **Positive:** Rationale survives contributor turnover and is reviewable in pull requests alongside the code that enacts it. Future "why did we do this?" questions have a canonical answer.
- **Positive:** Writing the ADR forces the trade-off to be made explicit before code is written.
- **Negative:** A small ongoing documentation cost; discipline is required to write ADRs for genuinely significant decisions and not for trivia.
- **Neutral:** ADRs are immutable history, so they will accumulate over time; the [index](./README.md) is the navigation aid.
