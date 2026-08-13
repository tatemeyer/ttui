# Release Governance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve `.claude/rules/code-forge.md`'s four-category stub
(labels, agent authorship, filing work, model-tiered dispatch) with a
concrete SemVer policy, a minimal label taxonomy, a triage/filing-work
rule, and formalization of already-working conventions — the process
the upcoming sweep audit (a separate, later sub-project) will use to
route what it finds.

**Architecture:** All-docs work converging on one file rewrite
(`code-forge.md`, stub → fully defined) plus two small additions
(`CHANGELOG.md`, a `README.md` sentence), and one GitHub-side action
(creating four labels). No source code, no TDD.

**Tech Stack:** Markdown, `gh label create` (GitHub CLI).

## Global Constraints

- **`admin`/`git-adjacent`-tagged, no TDD** — matches the existing tag
  conventions in `docs/design/README.md`; this is process/
  documentation work with zero `src/`/`examples/` behavior change.
- **SemVer applies to the `ttui` crate only** — `tools/visual-snapshot`
  stays outside this policy (internal dev tooling, no external
  consumers).
- **No version bump in this plan.** `Cargo.toml`'s `version` field is
  untouched here — that happens at the (separate, later) v1.0.0 tag
  step.
- **No new severity scale.** The four labels answer semver-impact and
  release-queue membership, not "how bad" — the existing Critical/
  Important/Minor review vocabulary is unchanged.
- **No new attribution mechanism.** Agent authorship is formalized via
  the existing `Co-Authored-By`/`Claude-Session` commit trailers,
  as-is — no new PR label.
- **No new model-dispatch scheme.** References
  `subagent-driven-development`'s existing "Model Selection" section
  rather than inventing a parallel one.
- **Direct autonomy tier applies** (`git-github-standards.md`): once
  this plan's tasks are done and reviewed, they may commit straight to
  `main` — no PR required.

---

### Task 1: Rewrite `code-forge.md`, add `CHANGELOG.md`, update `README.md`

**Files:**
- Modify: `.claude/rules/code-forge.md` (full rewrite)
- Create: `CHANGELOG.md`
- Modify: `README.md`

**Interfaces:** none — this is the plan's only doc-content task; Task
2 (labels) doesn't read anything from this task's file content, only
from the spec.

- [ ] **Step 1: Replace `code-forge.md` entirely**

Replace the full contents of `.claude/rules/code-forge.md` with:

```markdown
# Code Forge

**Status:** fully defined. See
`docs/design/specs/governance/2026-08-13-release-governance-design.md`
for the full rationale.

Governs how work gets labeled, attributed, filed, and routed to a
model tier.

## SemVer policy

Applies to the root `ttui` library crate only — `tools/visual-
snapshot` is internal dev tooling with no external consumers and stays
outside this policy.

**Breaking** means any change to `ttui`'s public API surface — the
same surface `src/lib.rs`'s `#![warn(missing_docs)]` already gates
(every `pub` item under `src/`): removing/renaming a `pub` item,
changing a `pub fn`'s signature, adding a required field to a `pub
struct`, changing a `pub` trait's required methods, or adding a
variant to an existing `pub enum` (none of `ttui`'s current public
enums are `#[non_exhaustive]`, so a new variant breaks an exhaustive
`match`). Adding a wholly new `pub` item, or a new optional
builder-pattern method on an existing type, is additive (minor).

This policy takes effect immediately for sizing findings —
`Cargo.toml`'s `version` field itself is not bumped until the actual
v1.0.0 tag cuts.

`CHANGELOG.md` (repo root) tracks changes in [Keep a
Changelog](https://keepachangelog.com) format, with entries accruing
under `## [Unreleased]` from now on.

## Labels

Four labels, deliberately minimal — answer "what's the semver impact
and release-queue status," not "how bad is it" (that's still the
existing Critical/Important/Minor review vocabulary):

| Label | Color | Description |
|---|---|---|
| `semver:patch` | `#cccccc` | Fix/change with no public API impact |
| `semver:minor` | `#0e8a16` | Adds public API surface, backward compatible |
| `semver:major` | `#d93f0b` | Breaking change to `ttui`'s public API surface |
| `v1-blocking` | `#fbca04` | Must land before the v1.0.0 tag |

## Filing work

Every finding (from a sweep audit or discovered organically) gets
filed as a GitHub issue first, even one about to be fixed immediately
— always a record.

**Triage:**
1. Touches `ttui`'s public API surface? Label `semver:minor` or
   `semver:major` **and** `v1-blocking` — free to fix now, costly
   after 1.0.
2. Otherwise, label `semver:patch`. No `v1-blocking` — post-v1 by
   default, unless trivial enough to fix along the way (see below).

**Routing the fix:**
- **Trivial, mechanical, well-understood** (stale doc comment,
  superseded helper, a one-line ordering bug with a clear root cause,
  a rename) — fixed directly, no subagent dispatch, referencing the
  issue in the commit and closing it.
- **Needs real design judgment** (new abstraction, behavior change
  with tradeoffs, anything warranting clarifying questions) — goes
  through the full `superpowers:brainstorming` → spec → plan →
  `subagent-driven-development` cycle.

## Agent authorship

Commits carry the `Co-Authored-By`/`Claude-Session` trailers already
established by the harness (see `development-conventions.md`'s commit
conventions). No additional PR-level label — the trailers are the
record.

## Model-tiered dispatch

Governed by `subagent-driven-development`'s "Model Selection" section
— cheap model for mechanical/transcription work, standard model for
integration/judgment tasks, most capable model for architecture and
final review. No separate, TTUI-specific scheme.

## Agent-authored work and merge gates

Resolved by `git-github-standards.md`'s Direct/Gated/Human autonomy
tiers: `admin`/`git-adjacent`-tagged work with zero `src/`/`examples/`
behavior change may commit straight to `main` (Direct);
`coding`/`research`-tagged work requires a PR with all four checks
green (Gated); branch-protection/CI-config changes, force-pushes, and
anything explicitly flagged as sensitive require human sign-off
(Human). See that file for the full tier definitions.
```

- [ ] **Step 2: Create `CHANGELOG.md`**

Create `CHANGELOG.md` at the repo root:

```markdown
# Changelog

All notable changes to `ttui` are documented in this file. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows the SemVer policy defined in
`.claude/rules/code-forge.md`.

## [Unreleased]

### Added

- Release governance: SemVer policy, label taxonomy, filing-work rule
  (`.claude/rules/code-forge.md`).
```

- [ ] **Step 3: Update `README.md`'s "## Workflow" section**

Find `README.md`'s existing "## Workflow" section:

```markdown
## Workflow

This project is developed using [superpowers](https://github.com/obra/superpowers)
— every feature goes through brainstorm → design doc → plan → subagent-
driven implementation. See `.claude/rules/` for project-specific
conventions layered on top of that.
```

Add one sentence to the end of the existing paragraph (do not rewrite
or split it into a new paragraph):

```markdown
## Workflow

This project is developed using [superpowers](https://github.com/obra/superpowers)
— every feature goes through brainstorm → design doc → plan → subagent-
driven implementation. See `.claude/rules/` for project-specific
conventions layered on top of that, including `.claude/rules/code-forge.md`
for how work is labeled, attributed, filed, and versioned.
```

- [ ] **Step 4: Verify**

Run these checks (no build/test — this is docs-only):

```bash
grep -c "stub\|open question" .claude/rules/code-forge.md
```
Expected: `0` (no leftover stub/open-question language).

```bash
grep -c "Unreleased" CHANGELOG.md
```
Expected: `1`.

```bash
grep -c "code-forge.md" README.md
```
Expected: `1` (the new sentence added in Step 3).

- [ ] **Step 5: Commit**

Per Global Constraints, this is Direct-tier (`admin`-tagged, zero
`src/`/`examples/` change) — commit straight to `main`, no PR. If
working from a worktree on its own feature branch (e.g.
`worktree-governance`), `git checkout main` won't work there if
another worktree already has `main` checked out (git refuses to check
out the same branch in two worktrees at once) — instead, commit
normally on the current branch, then push that commit directly onto
the remote `main` ref:

```bash
git add .claude/rules/code-forge.md CHANGELOG.md README.md
git commit -m "docs(governance): resolve code-forge.md stub, add CHANGELOG.md

Defines the SemVer policy, label taxonomy, filing-work rule, and
formalizes agent-authorship/model-dispatch conventions the upcoming
sweep audit will route findings through."
git push origin HEAD:main
```

---

### Task 2: Create the four GitHub labels

**Files:** none — this task only runs `gh label create` commands
against the repo; no file changes.

**Interfaces:**
- Consumes: the label names/colors/descriptions Task 1's rewritten
  `code-forge.md` documents (kept in sync — the values below are
  identical to what's in the file).

- [ ] **Step 1: Create each label**

Run these four commands (each is idempotent-safe to re-run if one
fails partway — `gh label create` errors clearly if a label already
exists, rather than silently duplicating):

```bash
gh label create "semver:patch" --color "cccccc" --description "Fix/change with no public API impact"
gh label create "semver:minor" --color "0e8a16" --description "Adds public API surface, backward compatible"
gh label create "semver:major" --color "d93f0b" --description "Breaking change to ttui's public API surface"
gh label create "v1-blocking" --color "fbca04" --description "Must land before the v1.0.0 tag"
```

- [ ] **Step 2: Verify all four exist with the right color/description**

```bash
gh label list --search "semver"
gh label list --search "v1-blocking"
```

Expected: `semver:patch` (`cccccc`), `semver:minor` (`0e8a16`),
`semver:major` (`d93f0b`) from the first command; `v1-blocking`
(`fbca04`) from the second. Descriptions match Step 1's exactly.

- [ ] **Step 3: Report**

No commit for this task (GitHub labels aren't repo files) — note in
the final verification (below) that all four labels were created and
verified.

## Final verification (whole plan)

- [ ] `.claude/rules/code-forge.md` contains no "stub" or "open
      question" language — all four original categories resolved.
- [ ] `CHANGELOG.md` exists with a `## [Unreleased]` section in Keep a
      Changelog format.
- [ ] `README.md`'s Workflow section links to `code-forge.md`.
- [ ] All four labels (`semver:patch`, `semver:minor`, `semver:major`,
      `v1-blocking`) exist in the repo with the colors/descriptions
      specified in Task 2.
- [ ] Per `git-github-standards.md`'s Direct tier: Task 1's commit
      landed straight on `main`, no PR — confirm via `git log
      --oneline -3` on `main` after pushing.
