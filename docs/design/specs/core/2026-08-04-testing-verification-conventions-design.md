# Testing & Verification Conventions — Design

**Status:** Draft, pending your review before we move to planning.
**Date:** 2026-08-04

## Context / Motivation

`.claude/rules/development-conventions.md` is a stub that explicitly
defers its testing-discipline questions until two prerequisites are
settled: the core language (Rust, decided in
`docs/design/specs/core/2026-08-04-ttui-core-framework-design.md`) and,
implicitly, how CI actually runs tests (decided in
`docs/design/specs/2026-08-04-gitops-github-workflow-design.md`, now
live — `cargo test` is a required check on every PR). Both
prerequisites are now resolved, unblocking this brainstorm.

This is **not** a re-design of the core framework's own testing
strategy — that's already specified in the core framework design doc
(inline unit tests for `buffer`/`layout`/`widgets`, an accepted gap for
the full event loop verified only by manual check, real-TTY tests
marked `#[ignore]`) and already written into that plan's tasks as
literal TDD test code. This spec instead answers
`development-conventions.md`'s open question — "what testing discipline
applies... beyond what `superpowers`' own `test-driven-development`
skill already provides" — as a **project-wide convention** that
applies to all future work, not just what's already speced for the
core framework.

## Scope

**In scope:** TDD enforcement policy and its exceptions, unit vs.
integration test structure, coverage tooling (or lack thereof), and how
real-TTY tests get verified in practice, as project-wide conventions.

**Explicitly out of scope:**
- Re-deciding anything the core framework spec/plan already settled
  about its own test content (widget snapshot-style assertions, buffer
  diffing tests, etc.) — those stand as written.
- Any new CI jobs or changes to `.github/workflows/ci.yml`. `cargo
  test`'s default behavior (skips `#[ignore]`'d tests) already matches
  this spec's real-TTY policy exactly, so nothing in CI needs to
  change.

## Decision: TDD enforcement

**Mandatory for all `coding`-tagged work** (per the Arc/Slice/Task tag
system in `docs/design/README.md`), with four named exceptions decided
up front — not left to case-by-case judgment on each plan:

1. **Pure config/git-adjacent work** — nothing to unit-test (e.g. the
   GitOps plan just executed: YAML/API configuration, no application
   logic).
2. **Examples/demos** — e.g. `examples/demo.rs` in the core framework
   plan. Correctness is checked by running the example, not asserting
   on it.
3. **Real-TTY/terminal-dependent code** — raw-mode enter/exit,
   panic-hook behavior, anything that can only be verified against a
   real terminal. Covered by the real-TTY policy below instead.
4. **`research`-tagged throwaway spikes** — exist to answer a question,
   get deleted or rewritten before they ship, so TDD overhead isn't
   worth it.

These four exceptions are documented as prose in
`development-conventions.md`, not as new tag values — the existing tag
set (`coding`, `research`, `admin`, `git-adjacent`) stays as-is,
consistent with `docs/design/README.md`'s stated preference to add tags
only when a genuinely new paradigm needs one.

Everything else tagged `coding` follows `superpowers:test-driven-
development` test-first, no exceptions beyond the four above.

## Decision: Test structure

Inline `#[cfg(test)] mod tests` per module stays the default — this
already matches every task in the core framework plan.

A top-level `tests/` integration directory is established, but not as
a standalone placeholder today: there's no `Cargo.toml` yet (the core
framework plan is still blocked), and an empty `tests/` directory isn't
a meaningful Rust artifact without a crate to test against. Instead,
**the core framework plan's Task 1** ("Initialize the Cargo project,"
which already creates `Cargo.toml` and `src/lib.rs`) is amended to also
create `tests/` at that same moment — a single placeholder file,
`tests/README.md` (documentation, not a compiled test — an empty `.rs`
file would compile into a no-op test binary, which is unnecessary
noise), establishing the convention from the crate's first commit, so
later work has a documented home for integration tests the moment one
is actually needed (specifically: a test that exercises the crate as
an external consumer would, via the public `ttui::` API across module
boundaries — not before).

## Decision: Coverage tooling

**None.** TDD-with-exceptions already means most code has tests by
construction; a tracked coverage percentage (via `cargo-tarpaulin` or
`cargo-llvm-cov`) adds CI complexity and a number to optimize toward,
without much signal beyond what TDD plus code review already provide.
Not revisited unless a concrete gap shows up in practice.

## Decision: Real-TTY test verification

**Permanently manual**, matching the core framework spec's already-
accepted gap — not "manual for now, revisit later." Standing policy:
before merging any PR that touches terminal/raw-mode code, run `cargo
test -- --ignored` locally and note the result in the PR template's
existing freeform Verification section. No PR template change needed —
the section already exists and is freeform; this is a documented
expectation for what goes in it, not new structure.

A self-hosted CI runner with real TTY access was considered and
rejected: real infrastructure/maintenance burden for a solo project,
not justified by the current gap. `cargo test`'s default exclusion of
`#[ignore]`'d tests means CI already does the right thing automatically
— no workflow change required to keep this policy in effect.

## Success criteria (this spec's "done")

- `.claude/rules/development-conventions.md` documents: the TDD policy
  and its four named exceptions, the test-structure decision
  (inline-by-default, `tests/` scaffolded via the core framework plan's
  Task 1), the no-coverage-tooling decision, and the real-TTY
  manual-verification policy.
- The core framework plan's Task 1 is amended to additionally create a
  `tests/` placeholder alongside `Cargo.toml`/`src/lib.rs`.
- No changes to `.github/workflows/ci.yml` (none needed).

## Explicitly deferred / open questions for future revisions

- None identified. Every question `development-conventions.md`'s stub
  raised about testing is resolved by this spec; its remaining open
  questions (commit granularity/message conventions) are unrelated to
  testing and out of scope here.
