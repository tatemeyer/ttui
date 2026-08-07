# Development Conventions — Design

**Status:** draft, pending your review.
**Date:** 2026-08-07
**Relationship to prior specs:** resolves the two open questions in
`.claude/rules/development-conventions.md` (coding style, commit
granularity/message conventions) and, per explicit scaling guidance,
extends into codebase/docs organization more broadly. Target scale for
these conventions: comfortably supporting the codebase growing to
roughly 50k lines of code and 200k lines of docs — those aren't goals,
they're the ceiling this needs to hold up under without a rewrite.

## Problem

`development-conventions.md` has been a stub since project bootstrap:
testing discipline is resolved (a separate spec), but coding style and
commit conventions were never formalized — despite ~40 commits and
three shipped example apps having already converged on real, consistent
habits worth writing down rather than reinventing. Separately, three
concrete scale problems have already appeared in miniature and will
only get worse: `examples/*.rs` files have grown past 1000 lines,
`docs/design/{specs,plans}/` are flat 17-entry directories that will
become unnavigable at hundreds of entries, and `src/` has zero doc
comments anywhere, which is fine at today's size but not at 50k LOC
where an agent doing cold discovery can't read every implementation.

## Scope

Six slices:

1. Formalize the existing commit convention (Conventional Commits, one
   commit per plan task) with one tightening: a required 1-2 sentence
   body on non-trivial `feat`/`fix` commits.
2. Establish agent-first doc-comment conventions (`//!` module headers,
   single-line `///` on public items) and retrofit them onto all
   current `src/` files, enforced going forward via `#![warn(missing_
   docs)]`.
3. Establish a directory-entry-count ceiling (~15-20 files) for `src/`
   subdirectories — a written rule only this round; nothing currently
   crosses it.
4. A 500-line soft ceiling per file, applied now by splitting all three
   `examples/*.rs` files into module directories.
5. Reorganize `docs/design/specs/` and `docs/design/plans/` into
   per-Arc subdirectories, with `docs/design/README.md` maintained as a
   living index of Arcs (not individual files).
6. An `examples/README.md` index describing what each app demonstrates.

**Explicitly out of scope:** restructuring `src/` or `src/widgets/`
themselves (both are under the new directory-entry-count ceiling —
slice 3 is a written rule for when they cross it, not a trigger this
round); per-Arc index files inside each new `docs/design/specs/<arc>/`
subdirectory (at 2-10 files per Arc, the directory listing itself is
navigable; an extra index file per Arc is busywork at this size);
rustdoc prose beyond single-line summaries (still no multi-paragraph
docstrings, per the standing no-comments-unless-non-obvious discipline
for everything below the module/item-header level).

## Design

### Slice 1: Commit conventions

Formalize exactly what's already in the git history since `d3e859d`:
`type(scope): description`, lowercase type (`feat`/`fix`/`docs`/
`chore`/`ci`/`test`), scope = crate area or app name, imperative mood,
trailing `(#N)` only when tied to a tracked issue, one commit per plan
task. New requirement: any `feat`/`fix` commit whose change isn't
self-evident from the subject line alone gets a 1-2 sentence body
stating *why* (motivation, tradeoff, or "per <spec file>" pointer) —
mechanical commits (`chore`, `docs` reformatting, `ci`, `fmt`-only)
stay subject-only. This is a documentation-only change to
`development-conventions.md` — no code, no TDD.

### Slice 2: Agent-first doc comments

Every `src/` module gets a `//!` header, 1-3 sentences: what the module
is, what it deliberately isn't (mirrors the existing spec-file
"Relationship to prior specs" habit of stating boundaries explicitly).
Every `pub struct`/`pub fn`/`pub enum` gets a single-line `///`
summary — purpose and usage, not a repeat of what the name already
says, no multi-paragraph prose. Example, matching this project's actual
style (terse, no filler):

```rust
//! Deterministic camera viewport and brightness scaling — used for
//! panning/zooming a rendered buffer and for boot-sequence fades.

/// A 2D viewport position and zoom level over a source `Buffer`.
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}
```

Inline WHY-only comments inside function bodies are unaffected by this
— still added only for genuinely non-obvious invariants (e.g.
`buffer.rs`'s existing transparency-rule comment), exactly as sparse as
today. Applied retroactively to every current `src/` file (not just
going forward): at 50k-LOC scale, a growing split between "old
undocumented code" and "new documented code" only gets more expensive
to close later. Enforced going forward by adding `#![warn(missing_
docs)]` to `src/lib.rs` — this turns the convention into a compiler
warning caught by the same `cargo clippy --all-targets -- -D warnings`
gate every task already runs, rather than relying on manual review.
This is doc-comment-only work with no behavior change — no unit-
testable surface, so it's TDD-exempt the same way the "pure config"
exception already covers other no-testable-behavior changes; verified
by `cargo doc` building clean and `missing_docs` warnings going to
zero, not by new tests.

### Slice 3: Directory-entry-count ceiling

Once a `src/` directory (or subdirectory) exceeds ~15-20 files, group
into thematic subdirectories with a `///` header on each new `mod.rs`
(e.g. `src/widgets/navigation/`, `src/widgets/meters/`, `src/widgets/
console/`, grouped by the Arc/theme each widget was built for). `src/`
top-level is at 13 files and `src/widgets/` is at 14 — both under
threshold, so this slice is a written rule only; nothing gets moved
this round. Written into `development-conventions.md` so the next
widget that crosses the line triggers a split as a matter of course,
not a judgment call made from scratch.

### Slice 4: File-size ceiling + example splits

500-line soft ceiling per file. All three examples already exceed it
(`omnitrix.rs` 850, `tardis.rs` 1005, `smash_crabs.rs` 1069 lines) and
get split now into Cargo's supported multi-file example layout:

```
examples/smash_crabs/
  main.rs        — struct, App impl (update/view/on_tick — a trait impl
                   must stay in one block, so these three dispatch
                   methods live here and match out to the per-screen
                   files below), shared consts, shared helpers used by
                   2+ screens (paint_background, shake_offset, blit,
                   render_row), RodioAudioSink, arena_theme
  hub.rs         — impl SmashCrabs { render_hub, hub_panels, ... }
  versus.rs      — paint_ui, paint_effects, render_versus, ...
  target_smash.rs
  stage_hazards.rs
  boot.rs
```

(same shape for `omnitrix/` split by `AppMode`, and `tardis/` split by
`Screen`). Multiple inherent `impl SmashCrabs` blocks across files is
ordinary Rust — only the single trait `impl App for SmashCrabs` block
must stay whole. This is example code, so it stays under the existing
TDD exception (verified by running each split app, not by new tests) —
each split is a pure code-motion with no behavior change, confirmed by
running the exact manual-verification checklist each app's original
implementation plan already specified.

### Slice 5: `docs/design/` reorganization

Per-Arc subdirectories, both under `specs/` and `plans/`:

```
docs/design/specs/
  core/          — ttui-core-framework, testing-verification-*, gitops-*,
                   buffer-layering-*, claude-audit-templates,
                   ttui-rev-b-vision-alignment, core-capabilities,
                   example-apps-roadmap, development-conventions (this one)
  omnitrix/      — omnitrix-dial-navigation-arc, omnitrix-faceplate,
                   omnitrix-glow-border, omnitrix-sub-apps-boot-arc
  smash-crabs/   — smash-crabs-arena-hub-arc, smash-crabs-remaining-sub-apps
  tardis/        — tardis-console-arc, tardis-remaining-sub-apps
```

(identical bucketing under `docs/design/plans/`). Moved via `git mv` to
preserve history — mechanical, `admin`/`git-adjacent`-tagged, no TDD.
`docs/design/README.md`'s filename-convention note updates to `specs/
<arc>/YYYY-MM-DD-<topic>-design.md`, and it gains a short "Arcs" list
(one line per subdirectory, updated whenever a genuinely new Arc
starts) — an index of *buckets*, not individual files, so it stays
small regardless of how many specs pile up inside each bucket.

### Slice 6: `examples/README.md`

A short index, one entry per app: name, one-sentence description of
what it demonstrates, and which vision doc it's built from. Exists so
"what does the TARDIS example show off" has an answer that doesn't
require opening the file.

## Verification

- `cargo test --lib`, `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings`, `cargo doc --no-deps` (must build clean with zero
  `missing_docs` warnings) all green.
- `cargo build --examples` clean after the three-way split; each split
  app's original manual-verification checklist (from its own
  implementation plan) re-run to confirm the split changed no behavior.
- `git log --follow` on at least one moved spec/plan file confirms
  history survived the `git mv`.
- `docs/design/README.md` and `examples/README.md` both read
  accurately against the post-reorg directory layout.
