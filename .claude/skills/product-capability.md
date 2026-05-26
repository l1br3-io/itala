# Product Capability Planning

Translate phase requirements, roadmap items, or feature discussions into
implementation-ready capability plans that expose constraints, invariants,
interfaces, and unresolved decisions before work starts.

## When to Use

- A phase exists in `docs/ROADMAP.md` but implementation constraints are implicit
- A feature touches multiple bounded contexts (kernel, editor, search, plugin host, etc.)
- A feature crosses the IPC boundary (Rust kernel ↔ webview shell)
- Product intent is clear but architecture, data, or lifecycle implications are fuzzy
- Hidden assumptions surface during review or implementation

## Workflow

### 1. Restate the Capability

Answer: **who** benefits, **what** changes, **what outcome** they get.

Reference `docs/PRODUCT.md` for personas and `docs/ROADMAP.md` for phase scope.
Cross-check with `.claude/memories/phase-status.md` for current state.

### 2. Resolve Constraints

For each capability, enumerate:

| Category | Questions to Answer |
|---|---|
| **Business Rules** | What is always true? What is never allowed? |
| **Scope** | What is this capability? What is it NOT? |
| **Data Ownership** | Which crate owns this data? Where is it persisted (vault Markdown, `.itala/index.redb`, opendal-backed transport)? |
| **Lifecycle** | Created when? Updated how? Deleted/archived when? CRDT semantics? |
| **Failure Modes** | What happens when the plugin host is unavailable? When the AI provider rate-limits? When sync transport is offline? When the search index is corrupted? |
| **Growth Boundary** | At what scale or complexity does this design need revisiting? What future requirement would invalidate the core design choice? What are we explicitly not solving that a future capability might assume we solved? |
| **Rollout** | Phase dependency? Feature flag? Migration required (data, plugin API, schema)? |

### 3. Define Implementation Contract

Produce an SRS-style plan covering:

- **Actors**: User, kernel, plugin host, plugins, AI provider, sync transport, webview shell
- **Surfaces**: Tauri command, palette action, context trait, event-bus topic, WIT interface
- **States/Transitions**: What states does the entity go through? What CRDT operations apply?
- **Interfaces**:
  - Rust trait signatures (`pub trait X { async fn ... }`)
  - WIT interfaces if plugin-facing (`interface ... { ... }`)
  - Tauri commands (`#[tauri::command] async fn ...`) with typed args
  - Solid component props (`interface ...Props { ... }`)
- **Data**: Newtype IDs, frontmatter keys, redb table changes, postcard envelopes, yrs `Map`/`Text` slots
- **Security**: Input validation at boundaries, capability declarations for plugins, secret handling, encryption choices
- **Observability**: Structured `tracing` events, error variants, user-visible failure surfaces
- **Open Questions**: Anything that needs a product decision before implementation

### 4. Translate to Execution

Decide the handoff:
- **Ready for /plan**: Constraints resolved, interfaces defined, proceed to implementation planning
- **Needs review**: Constraints mostly resolved, but some trade-offs need discussion
- **Needs clarification**: Open questions block implementation — list them explicitly

## Output Format

```markdown
# Capability: [Name]

## CAPABILITY
[One-paragraph restatement: who, what, outcome]

## CONSTRAINTS
- [Fixed rule or boundary — always true]
- [Fixed rule or boundary — never allowed]

## IMPLEMENTATION CONTRACT

### Actors
- [Actor]: [Role in this capability]

### Surfaces
- [Tauri command / palette action / WIT interface / kernel trait / event topic]

### States & Transitions
- [State 1] → [State 2]: [trigger, CRDT op if applicable]

### Interfaces
- `pub trait Foo { async fn bar(&self, ...) -> Result<...> }` — [purpose]
- WIT: `interface foo { bar: func(...) -> result<..., string> }`
- `#[tauri::command] async fn bar(...) -> Result<..., AppError>` — [purpose]
- `<NotePreview note={...} onSelect={...} />` — [purpose]

### Data
- [Newtype IDs introduced]
- [redb table additions]
- [frontmatter keys]
- [yrs CRDT slots]

### Security
- [Validation at this boundary]
- [Capability requirements for plugin-facing surface]

### Observability
- [Tracing fields]
- [Error variants the user sees]

## NON-GOALS
- [What this capability explicitly does NOT cover]

## DESIGN BOUNDARIES
- [Scale limit: what breaks at 10x current load or complexity, and what specific condition triggers a revisit]
- [Invalidating requirement: what single new need would make the core design choice wrong]
- [Assumed non-goals: what this explicitly does not solve that a future capability might assume it does]

## OPEN QUESTIONS
- [Blocker or product decision needed]

## HANDOFF
- [Ready for /plan | Needs review | Needs clarification]
- [Which agent/command takes it next]
```

## Rules

1. Do not invent product truth — mark unresolved questions explicitly
2. Separate user-visible promises from implementation details
3. Call out what is fixed (constraint) vs. preference (could change) vs. open (needs decision). For every significant design choice, explicitly state what future requirement would make it wrong.
4. Always check both kernel and webview impact for user-facing features
5. Reference the kernel's repository pattern (in `rules/api-design.md`) for collection contracts
6. Respect the local-first principle — cloud features (sync, cloud AI) must degrade gracefully
7. Plugin-facing capabilities go through the WIT contract, never around it
