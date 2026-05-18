# Project Flow Ops

Operate execution flow on GitHub by triaging issues and PRs, classifying work,
and keeping the backlog actionable.

## When to Use

- Triage open issue or PR backlogs
- Classify PRs: merge, rebuild, close, or park
- Audit review comments, CI failures, or stale issues
- Prioritize work across phases
- Clean up after a milestone or release

## Operating Model

- **GitHub** is the single source of truth (no Linear — Itala is open-source)
- Not every issue needs immediate action — park what's not scheduled
- Labels drive priority — use phase labels that match `docs/ROADMAP.md` (e.g. `phase/foundation`, `phase/desktop`, `phase/search`, `phase/plugin-host`, `phase/cli`, `phase/curated-plugins`, `phase/post-mvp`) plus type labels (`bug`, `enhancement`, `good-first-issue`)
- Milestones map to roadmap phases

## Core Workflow

### 1. Read the Surface

For each issue or PR, gather:
- State (open/closed/draft)
- Author and branch status
- Labels and milestone
- Review comments and CI status
- Linked issues or PRs
- Whether it touches the kernel, the webview shell, the plugin host, or docs

### 2. Classify the Work

| Classification | Meaning | Action |
|---|---|---|
| **Merge** | Self-contained, tested, policy-compliant, ready | Approve and merge |
| **Rebuild** | Useful idea, but implementation needs rework | Close with guidance, open a new issue scoped correctly |
| **Close** | Wrong direction, stale, duplicate, or superseded | Close with explanation |
| **Park** | Potentially useful, not scheduled now | Label `parked`, remove from milestone |

### 3. Prioritize by Phase

Current priority order follows `.claude/memories/phase-status.md`. As of the active phase:
1. Items labelled with the in-progress phase
2. Items labelled with the next-up phase
3. Bug fixes against any shipped phase
4. Post-MVP items remain parked unless explicitly pulled in

### 4. Keep State Consistent

When work ships:
- Close the issue with a reference to the commit/PR
- Update labels and milestone
- If it affects phase status, update `.claude/memories/phase-status.md` and run `/sprint-update`

When work is rejected:
- Close with clear explanation
- If the idea has merit, open a new issue scoped correctly

## Review Rules

- Never merge from title alone — read the full diff
- CI red = investigate and fix or block; don't merge broken code
- External contributions should be rebuilt inside the project's patterns when valuable
- If a PR conflicts with `docs/VISION.md` or `docs/ARCHITECTURE.md`, say so explicitly

## Output Format

```text
ISSUE/PR: #N — [title]

STATUS
- State: open/closed/draft
- CI: passing/failing/none
- Reviews: approved/changes-requested/none
- Surface: kernel / plugin-host / webview / docs / multi

CLASSIFICATION
- merge / rebuild / close / park
- [One-paragraph rationale]

PRIORITY
- Phase: [phase label]
- Urgency: now / next sprint / backlog

NEXT ACTION
- [Exact next step — e.g., "approve and merge", "request changes: missing tests",
  "close: superseded by #M", "label parked, remove from milestone"]
```

## Bulk Triage Template

```markdown
# Backlog Triage — [date]

## Summary
- Total items: N
- Merge: N
- Rebuild: N
- Close: N
- Park: N

## Items
| # | Title | Classification | Phase | Action |
|---|---|---|---|---|
| 1 | ... | merge | foundation | approve |
| 2 | ... | park | - | label parked |
```
