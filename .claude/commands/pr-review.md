---
description: Code review — local uncommitted changes or GitHub PR (pass PR number/URL for PR mode)
argument-hint: [pr-number | pr-url | blank for local review]
---

# Code Review

Orchestrates a code review and, in PR mode, publishes it to GitHub. The actual analysis is delegated to the [`code-reviewer`](../agents/code-reviewer.yaml) agent, which is Itala-stack-aware (Rust / TypeScript / WIT / docs+config buckets, with the matching `rules/*` loaded per bucket).

**Input:** `$ARGUMENTS`

## Mode selection

If `$ARGUMENTS` is a PR number, a PR URL, or starts with `--pr` → **PR Review Mode**.
Otherwise → **Local Review Mode**.

---

## Local Review Mode

Review uncommitted changes before they leave the working tree.

1. **Gather** the diff:
   ```bash
   git diff --staged
   git diff
   ```
   If both are empty, stop: `Nothing to review.`

2. **Delegate** to the `code-reviewer` agent. Pass it the diff. It will classify files by bucket, load the matching `.claude/rules/*` and `rules/rust/*`, and produce a severity-ordered report per its own output format.

3. **Run** local validation if the repo has a `Cargo.toml`:
   ```bash
   just ci
   ```
   If pre-runtime (no `Cargo.toml` yet), skip and note it.

4. **Report** the agent's findings to the user. Block on any CRITICAL.

---

## PR Review Mode

### Phase 1 — Fetch

Resolve `$ARGUMENTS` to a PR number:

| Input | Action |
|---|---|
| Number (`42`) | use as PR number |
| URL (`github.com/.../pull/42`) | extract the number |
| Branch name | `gh pr list --head <branch>` |

```bash
gh pr view <N> --json number,title,body,author,baseRefName,headRefName,changedFiles,additions,deletions
gh pr diff <N>
```

If the PR doesn't exist, stop with the gh error.

### Phase 2 — Context

Build the review context:

1. Load `CLAUDE.md` and the relevant `.claude/rules/*` (the `code-reviewer` agent does this per bucket — don't pre-load globally).
2. Check `.claude/plans/<id>.md` for any plan whose `id` matches the PR's linked feature; if present, the plan's success criteria become part of the review.
3. Parse the PR description for goals, linked issues, and the PR author's test plan.

### Phase 3 — Review

Delegate to the `code-reviewer` agent against the PR diff. The agent will:
- Classify changed files into buckets (Rust / TypeScript / WIT / docs+config).
- Load only the rule files that govern those buckets.
- Apply Itala-specific checklists per `rules/rust/*` and `rules/typescript/*`.
- Return a severity-ordered report with the agent's own output format.

### Phase 4 — Validate

Run the gates that apply at the PR head revision:

```bash
just ci    # fmt → clippy → nextest → deny → doc
```

If the diff is docs/config only (no Rust or TS changed), skip and note it. Record pass/fail of each step.

### Phase 5 — Decide

| Condition | Decision |
|---|---|
| Zero CRITICAL/HIGH, `just ci` green | **APPROVE** |
| Only MEDIUM/LOW, `just ci` green | **APPROVE** with comments |
| Any HIGH or `just ci` failure | **REQUEST CHANGES** |
| Any CRITICAL | **BLOCK** |
| Draft PR | **COMMENT** only (regardless of severity) |

If the `code-reviewer` agent flagged a security escalation, append: *"Recommend invoking `security-reviewer` for deeper analysis."*

### Phase 6 — Publish

Post to GitHub:

```bash
# APPROVE
gh pr review <N> --approve --body "<summary>"

# REQUEST CHANGES
gh pr review <N> --request-changes --body "<summary with required fixes>"

# COMMENT (draft PRs, informational)
gh pr review <N> --comment --body "<summary>"
```

For inline comments at specific lines, batch via the reviews API:

```bash
gh api "repos/{owner}/{repo}/pulls/<N>/reviews" \
  -f event="COMMENT" \
  -f body="<overall summary>" \
  --input comments.json   # [{"path": "...", "line": N, "body": "..."}, ...]
```

### Phase 7 — Output

Report to the user:

```
PR #<N>: <title>
Decision: <APPROVE | REQUEST_CHANGES | BLOCK | COMMENT>
Findings: <C> CRITICAL, <H> HIGH, <M> MEDIUM, <L> LOW
Validation: just ci <pass|fail|skipped>
GitHub: <PR URL>
```

---

## Edge cases

- **No `gh` CLI** — fall back to local-only review and warn.
- **Diverged branches** — suggest `git fetch origin && git rebase origin/<base>` before reviewing.
- **Pre-runtime repo state** — if `crates/` doesn't exist yet, the diff is expected to be docs/config only. Don't fabricate Rust findings; the `code-reviewer` agent's pre-runtime note governs.
- **Large PRs (>50 files)** — warn about scope. Have the agent prioritize source changes, then tests, then config/docs.

## Related

- [`code-reviewer` agent](../agents/code-reviewer.yaml) — the stack-aware reviewer that does the analysis.
- [`security-reviewer` agent](../agents/security-reviewer.yaml) — escalate when the diff touches `unsafe`, FFI, crypto, IPC, plugin host, or signature verification.
- [`/workflow`](workflow.md) — full pipeline context.
