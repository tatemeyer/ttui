# Development Conventions

**Status:** fully defined. See
`docs/design/specs/2026-08-07-development-conventions-design.md` for
the full rationale behind coding style, commit conventions, and the
file/directory/docs organization rules below.

## Scope

General day-to-day engineering conventions for this repo: coding style,
testing expectations, commit hygiene, file/directory organization, and
anything else that applies across the whole codebase regardless of
language.

## Testing

Core language is Rust (`docs/design/specs/2026-08-04-ttui-core-framework-design.md`).

**TDD is mandatory for all `coding`-tagged work** (per the
Arc/Slice/Task tag system in `docs/design/README.md`), via
`superpowers:test-driven-development`, with four named exceptions:

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
`tests/` integration directory is scaffolded by the core framework
plan's Task 1 (not yet executed — that plan is still blocked) for the
moment a test needs to exercise the crate as an external consumer
would, via the public `ttui::` API across module boundaries — not
before.

**Coverage tooling:** none. TDD-with-exceptions already means most code
has tests by construction; a tracked coverage percentage adds CI
complexity without much added signal. Not revisited unless a concrete
gap shows up in practice.

**Real-TTY tests:** permanently manual — not "manual for now." Before
merging any PR touching terminal/raw-mode code, run
`cargo test -- --ignored` locally and note the result in the PR
template's existing freeform Verification section. `cargo test`'s
default exclusion of `#[ignore]`'d tests already makes CI do the
right thing automatically; no CI workflow change is needed to keep
this policy in effect. A self-hosted runner with real TTY access was
considered and rejected — infrastructure/maintenance burden not
justified for a solo project.

Full rationale: `docs/design/specs/2026-08-04-testing-verification-conventions-design.md`.

## Commit conventions

Conventional Commits: `type(scope): description`.

- **Type:** lowercase — `feat`, `fix`, `docs`, `chore`, `ci`, `test`.
- **Scope:** the crate area or app name touched — `core`, `widgets`,
  `omnitrix`, `tardis`, `smash_crabs`, or a specific module/capability
  name (`camera`, `glitch`, `design` for docs-only commits).
- **Subject:** imperative mood ("add X", not "added X" or "adds X").
- **Issue reference:** trailing `(#N)` only when the commit is tied to
  a tracked GitHub issue; omitted otherwise (e.g. widget-only commits,
  docs commits with no issue).
- **Body:** required, 1-2 sentences, on any `feat`/`fix` commit whose
  change isn't self-evident from the subject alone — state *why*
  (motivation, tradeoff, or a pointer to the driving spec). Mechanical
  commits (`chore`, `docs` reformatting, `ci`, `fmt`-only) stay
  subject-only.
- **Granularity:** one commit per implementation-plan task.
- **Agent attribution:** agent-authored commits carry the
  `Co-Authored-By`/`Claude-Session` trailers already established by
  the harness — no separate convention needed here.

## Doc comments

Agent-first, not exhaustive rustdoc:

- Every `src/` module gets a `//!` header, 1-3 sentences: what the
  module is, what it deliberately isn't.
- Every `pub struct`/`pub fn`/`pub enum` in `src/` gets a single-line
  `///` summary — purpose and usage, not a restatement of the name.
  No multi-paragraph prose.
- Inline comments inside function bodies stay exactly as sparse as
  today: only for a genuinely non-obvious invariant, workaround, or
  subtlety. This rule does not loosen that discipline.
- Enforced via `#![warn(missing_docs)]` in `src/lib.rs`, caught by the
  same `cargo clippy --all-targets -- -D warnings` gate every task
  already runs.
- `examples/*.rs` are exempt — they're demos meant to be read
  start-to-finish, not a library API surface.

## File and directory size

- **Soft ceiling: 500 lines per file.** When a file crosses it, split
  by natural boundaries (one file per screen/mode for example apps,
  one file per responsibility for `src/`).
- **Soft ceiling: ~15-20 files per directory.** When a `src/`
  directory crosses it, group into thematic subdirectories (e.g.
  `src/widgets/navigation/`, `src/widgets/meters/`), each with a `///`
  header on its `mod.rs`.
- Multiple example-app screens can share one `struct`/`impl App` block
  in a `main.rs` entry point while their per-screen rendering methods
  live in separate files as additional inherent `impl` blocks — this
  is ordinary, supported Rust (only a single trait `impl` block is
  required to stay whole).

## Docs organization

- `docs/design/specs/` and `docs/design/plans/` are organized into
  per-Arc subdirectories (`core/`, `omnitrix/`, `tardis/`,
  `smash-crabs/`, and new ones as new Arcs start) rather than one flat
  directory. Filename convention: `specs/<arc>/YYYY-MM-DD-<topic>-
  design.md` (and the equivalent under `plans/`).
- `docs/design/README.md` is maintained as a living index of *Arcs*
  (one line per subdirectory), not individual files — this keeps the
  index small regardless of how many specs/plans exist within each
  Arc.
- `examples/README.md` indexes each example app: name, one-sentence
  description, and the vision doc it's built from.
