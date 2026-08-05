# Testing & Verification Conventions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Structure note:** This plan is organized as **Arcs → Slices → Tasks**
> per `docs/design/README.md`, not the flat "Task N" list the
> `writing-plans` skill defaults to. Arc/Slice headings are pure
> grouping; tasks still follow the skill's bite-sized step structure,
> adapted for documentation-only work (no unit-testable code here —
> both tasks are pure `admin`/`git-adjacent` doc edits, one of this
> plan's own named TDD exceptions).

**Goal:** Resolve `.claude/rules/development-conventions.md`'s
testing-discipline open question, and scaffold a `tests/` placeholder
into the core framework plan's Task 1, per
`docs/design/specs/2026-08-04-testing-verification-conventions-design.md`.

**Architecture:** No application architecture — this plan edits two
existing markdown documents. No code changes, no CI changes (the spec
explicitly requires none).

**Tech Stack:** N/A (documentation only).

## Global Constraints

- TDD is mandatory for all `coding`-tagged work, with exactly four
  named exceptions: pure config/git-adjacent work, examples/demos,
  real-TTY/terminal-dependent code, and `research`-tagged throwaway
  spikes. No new tags are introduced — the existing set (`coding`,
  `research`, `admin`, `git-adjacent`) stays as-is.
- Inline `#[cfg(test)] mod tests` per module is the default test
  structure. A `tests/` integration directory is scaffolded via the
  core framework plan's Task 1 (not created standalone before
  `Cargo.toml` exists), with a single non-compiled placeholder file,
  `tests/README.md`.
- No coverage tooling.
- Real-TTY tests (`#[ignore]`'d) are verified permanently manually via
  `cargo test -- --ignored`, noted in the PR template's existing
  freeform Verification section. No CI workflow changes.
- Both tasks in this plan are documentation edits only — no test code
  applies to either (matches this same plan's "pure config/git-adjacent
  work" TDD exception).

---

## Arc 1: Development Conventions Documentation

### Slice 1.1: Testing discipline section

**Tags:** admin, git-adjacent

#### Task 1: Write the testing section into `development-conventions.md`

**Files:**
- Modify: `.claude/rules/development-conventions.md`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: a documented, citable testing-discipline convention at
  `.claude/rules/development-conventions.md` — the file any future
  brainstorm/plan checks before proposing testing-related work, per
  root `CLAUDE.md`'s instruction to read `.claude/rules/` before
  proposing any design.

- [ ] **Step 1: Replace the file's full contents**

The current file (19 lines) is a stub whose "core language not yet
chosen" bullet is now stale (Rust was decided in the core framework
design doc) and whose testing question this task resolves. Replace the
entire file with:

```markdown
# Development Conventions

**Status:** partially defined — testing discipline resolved below (see
`docs/design/specs/2026-08-04-testing-verification-conventions-design.md`).
Coding style and commit granularity/message conventions are still
open, pending a `/superpowers:brainstorm` pass.

## Scope

General day-to-day engineering conventions for this repo: coding style,
testing expectations, commit hygiene, and anything else that applies
across the whole codebase regardless of language.

## Testing

Core language is Rust (`docs/design/specs/2026-08-04-ttui-core-framework-design.md`).

**TDD is mandatory for all `coding`-tagged work** (per the Arc/Slice/Task
tag system in `docs/design/README.md`), via `superpowers:test-driven-
development`, with four named exceptions:

- **Pure config/git-adjacent work** — nothing to unit-test (e.g. YAML/API
  configuration, no application logic).
- **Examples/demos** — e.g. `examples/demo.rs` in the core framework
  plan. Correctness is checked by running the example, not asserting on
  it.
- **Real-TTY/terminal-dependent code** — raw-mode enter/exit, panic-hook
  behavior, anything only verifiable against a real terminal. See
  "Real-TTY tests" below instead.
- **`research`-tagged throwaway spikes** — exist to answer a question,
  get deleted or rewritten before they ship.

These exceptions are the only ones — they don't extend the tag set
(`coding`, `research`, `admin`, `git-adjacent` stays as-is); everything
else tagged `coding` follows TDD test-first, no case-by-case exceptions
beyond the four above.

**Test structure:** inline `#[cfg(test)] mod tests` per module is the
default (matches every task in the core framework plan). A top-level
`tests/` integration directory exists (scaffolded via the core
framework plan's Task 1) for the moment a test needs to exercise the
crate as an external consumer would, via the public `ttui::` API across
module boundaries — not before.

**Coverage tooling:** none. TDD-with-exceptions already means most code
has tests by construction; a tracked coverage percentage adds CI
complexity without much added signal. Not revisited unless a concrete
gap shows up in practice.

**Real-TTY tests:** permanently manual — not "manual for now." Before
merging any PR touching terminal/raw-mode code, run `cargo test --
--ignored` locally and note the result in the PR template's existing
freeform Verification section. `cargo test`'s default exclusion of
`#[ignore]`'d tests already makes CI do the right thing automatically;
no CI workflow change is needed to keep this policy in effect. A
self-hosted runner with real TTY access was considered and rejected —
infrastructure/maintenance burden not justified for a solo project.

Full rationale: `docs/design/specs/2026-08-04-testing-verification-conventions-design.md`.

## Open questions to resolve via brainstorming

- Coding style conventions (formatting beyond `cargo fmt`, naming,
  module organization norms).
- Commit granularity and message conventions.
```

- [ ] **Step 2: Verify the replacement**

Run: `grep -c "TDD is mandatory" .claude/rules/development-conventions.md`
Expected: `1`

Run: `grep -c "Core language(s) for TTUI itself are not yet chosen" .claude/rules/development-conventions.md`
Expected: `0` (the stale bullet is gone)

- [ ] **Step 3: Commit**

```bash
git add .claude/rules/development-conventions.md
git commit -m "docs(dev-conventions): resolve testing-discipline open question"
```

---

## Arc 2: Core Framework Plan Amendment

### Slice 2.1: Scaffold `tests/` in the core framework plan's Task 1

**Tags:** admin, git-adjacent

#### Task 2: Amend the core framework plan to create `tests/README.md` alongside `Cargo.toml`

**Files:**
- Modify: `docs/design/plans/2026-08-04-ttui-core-framework-plan.md`

**Interfaces:**
- Consumes: nothing from Task 1 of this plan (independent edit, same
  spec).
- Produces: an amended core framework plan whose Task 1, once executed,
  creates `tests/README.md` alongside `Cargo.toml`/`src/lib.rs`/
  `.gitignore` — establishing the `tests/` convention from the crate's
  first commit, per this plan's Arc 1 documentation.

- [ ] **Step 1: Locate Task 1 in the core framework plan**

In `docs/design/plans/2026-08-04-ttui-core-framework-plan.md`, find
`#### Task 1: Initialize the Cargo project`. Its **Files** list
currently reads:

```markdown
**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `.gitignore` (Rust section: `/target`)
```

- [ ] **Step 2: Add `tests/README.md` to the Files list**

Replace that block with:

```markdown
**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `.gitignore` (Rust section: `/target`)
- Create: `tests/README.md`
```

- [ ] **Step 3: Update the Interfaces > Produces line**

The task's **Interfaces** block currently reads:

```markdown
**Interfaces:**
- Consumes: nothing (first task)
- Produces: an empty `ttui` library crate that later tasks add modules to.
```

Replace the `Produces` line with:

```markdown
- Produces: an empty `ttui` library crate that later tasks add modules
  to, plus a `tests/` integration-test placeholder (see
  `docs/design/specs/2026-08-04-testing-verification-conventions-design.md`).
```

- [ ] **Step 4: Insert a new step creating `tests/README.md`**

The task currently has 5 steps (Create `Cargo.toml`, Create
`src/lib.rs`, Create `.gitignore`, Verify the crate builds, Commit).
Insert a new step after the existing "Step 3: Create `.gitignore`"
step and before "Step 4: Verify the crate builds," renumbering the
following two steps to Step 5 and Step 6. The new step's heading line
is `- [ ] **Step 4: Create \`tests/README.md\`**`, followed by its
content block:

```markdown
# Integration tests

Not used yet. Unit tests live inline via `#[cfg(test)] mod tests` in
each module — see
`docs/design/specs/2026-08-04-testing-verification-conventions-design.md`.
This directory is for integration tests that exercise the crate as an
external consumer would, via the public `ttui::` API across module
boundaries. Add a test file here the first time one is actually
needed, not before.
```

- [ ] **Step 5: Renumber and update the verify step**

The old "Step 4: Verify the crate builds" becomes "Step 5," unchanged
in content (`cargo build` doesn't need `tests/README.md` to succeed —
it's not Rust source).

- [ ] **Step 6: Renumber and update the commit step**

The old "Step 5: Commit" becomes "Step 6." Update its `git add` command
to include the new file:

```bash
git add Cargo.toml Cargo.lock src/lib.rs .gitignore tests/README.md
git commit -m "chore: initialize ttui crate with crossterm dependency"
```

- [ ] **Step 7: Verify the amendment**

Run: `grep -c "tests/README.md" docs/design/plans/2026-08-04-ttui-core-framework-plan.md`
Expected: `3` (the Files list entry, the new step's heading, and the
commit command — the Produces line references `tests/` generically and
doesn't repeat the filename, so it doesn't add to this count)

Run: `grep -c "^#### Task 1:" docs/design/plans/2026-08-04-ttui-core-framework-plan.md`
Expected: `1` (confirms no duplicate Task 1 heading was accidentally introduced)

- [ ] **Step 8: Commit**

```bash
git add docs/design/plans/2026-08-04-ttui-core-framework-plan.md
git commit -m "docs(core-framework-plan): scaffold tests/ in Task 1"
```
