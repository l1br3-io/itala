# ADR-0007 — Bounded best-effort events with explicit reconciliation

- **Status:** Accepted
- **Date:** 2026-07-19

## Context

The kernel event bus uses a bounded `tokio::sync::broadcast` channel. Earlier documents called its delivery guarantee "at least once," but a slow subscriber can fall behind the retained capacity and receive a lag error. The bus has no durable log, acknowledgement protocol, retry queue, or consumer offset, so it cannot provide at-least-once delivery across lag or process restarts.

The bounded design is still appropriate for responsive in-process notifications. It prevents unbounded memory growth and keeps contexts decoupled. The contract must describe what the mechanism actually guarantees and how consumers recover.

## Decision

1. The event bus provides **best-effort delivery to active subscribers** through bounded queues.
2. Falling behind is explicit: subscribers receive a lag condition rather than silently assuming they observed every event.
3. Events are notifications, not the source of truth and not a durable work queue. A producer persists canonical state before publishing the corresponding event.
4. A lagged or newly started consumer reconciles from canonical Markdown or its domain's durable state. Search and graph consumers may rebuild their derived indexes.
5. Handlers are idempotent. Duplicate notifications, repeated reconciliation, and full rebuilds are valid behavior.
6. Operations that require a durable guarantee use direct shell orchestration or a purpose-built durable port rather than relying on the event bus.

## Consequences

- **Positive:** The documented guarantee matches the implemented broadcast primitive.
- **Positive:** Bounded memory use is retained.
- **Positive:** Derived contexts gain an explicit corruption and lag recovery strategy.
- **Negative:** Consumers must implement reconciliation and test lag behavior.
- **Negative:** The bus cannot be used for durable background jobs without introducing a separate mechanism.
- **Neutral:** If future requirements demand at-least-once delivery, a durable event log or queue will require a new ADR rather than a documentation-only change.
