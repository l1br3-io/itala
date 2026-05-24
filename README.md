# Itala — Knowledge Management System

**Your knowledge, your way. Extend everything.**

Itala is a cross-platform, extensible knowledge management system for notes, links, and ideas. It provides a flexible workspace where users can create, connect, and retrieve information across devices. The system is built around **modularity**: every capability—editing, search, sync, AI, PDF handling—is a plugin. The core is a thin orchestration layer that provides shared primitives and lets plugins compose the full experience.

---

## Quickstart

> Itala is in active design. Install instructions and CLI usage will land here as the first release takes shape. See [docs/ROADMAP.md](docs/ROADMAP.md) for what's being built now.

---

## Learn more

- **[docs/VISION.md](docs/VISION.md)** — Design orientation and core principles.
- **[docs/PRODUCT.md](docs/PRODUCT.md)** — Who Itala is for and the jobs they want to accomplish.
- **[docs/ROADMAP.md](docs/ROADMAP.md)** — MVP scope, phased delivery, and future vision.
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — How Itala works under the hood: kernel, plugin system, data model, security, accessibility.

---

## Contributing

The canonical development commands are driven by [`just`](https://github.com/casey/just):

```sh
just          # lint + test (default)
just lint     # fmt check + clippy
just test     # nextest + doctests
just deny     # license and supply-chain check
just doc      # build docs (broken-link check)
```

Install the toolchain (pinned in `rust-toolchain.toml`) with `rustup`, then `cargo install just` and `cargo install cargo-nextest cargo-deny` to get started.
