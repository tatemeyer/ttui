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
