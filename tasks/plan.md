# Plan — M2: Write Engine (current phase)

> **Status:** Not yet planned. This is a stub holding the current-phase slot.
> Spec: [`../SPEC.md`](../SPEC.md) (root, draft).

## Resuming

Nothing yet — the M2 spec is draft and no features are in flight.

## Next

1. Refine [`SPEC.md`](../SPEC.md) via `/spec` (acceptance criteria, scope boundaries).
2. Run `/planning` to decompose M2 into phased features; that command replaces this stub
   with a real plan and populates [`todo.md`](./todo.md).
3. `/build` → `/test` → `/review` → `/ship`.

## Context

- M0 (Foundation) and M1 (Vaults & Notes) shipped — see [`archive/`](./archive/).
- M2 builds the document engine on top of the kernel + vault landed in M0/M1.
- Markdown stays canonical (ADR-0006); the document model is subordinate and must round-trip.
