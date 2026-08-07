# Development Conventions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-07-development-conventions-design.md`:
formalize commit/doc-comment/file-size/directory-organization
conventions, retrofit doc comments onto all current `src/` files,
reorganize `docs/design/` into per-Arc subdirectories, split all three
`examples/*.rs` apps into module directories, and add an
`examples/README.md` index.

**Architecture:** Eight tasks, mostly independent of each other (no
task's `old_string` edits depend on another task having run first,
except that Task 4's `git mv` moves *this plan file itself* and its
spec, which must happen after Tasks 1-3, 5-8 have already committed
their own changes against the *current* flat paths). Order: Task 1
(rules doc) → Tasks 2-3 (doc comments) → Tasks 5-7 (example splits) →
Task 8 (examples index) → Task 4 (docs reorg, last, since it moves
every spec/plan file that exists by that point, including ones from
this very session).

**Tech Stack:** Rust, `cargo doc`, `git mv`.

## Global Constraints

- Every task here is either pure documentation or a pure, behavior-
  preserving code motion — none of it is TDD-tagged `coding` work in
  the sense that requires failing-test-first: doc comments have no
  testable behavior (verified by `cargo doc` + the `missing_docs`
  lint), and the example-app splits are `examples/*.rs` changes
  (already-established TDD exception, verified by running each app's
  existing manual-verification checklist).
- **Split-app constraint, applies to Tasks 5-7:** all `const`/`enum`/
  `struct` declarations stay in each app's `main.rs` (crate root) —
  only `fn`/method *bodies* move to per-screen files. This avoids
  cross-module qualification churn for the many consts each `new()`
  constructor reads. Each per-screen file starts with `use super::*;`
  to inherit `main.rs`'s imports, consts, and the struct definition
  without re-declaring anything.
- Methods moved into a per-screen file's own `impl SomeStruct { ... }`
  block need `pub(crate)` visibility — a private `fn` is only visible
  to its defining module and that module's *descendants*, not to
  ancestors like the crate root that need to call it from `update()`/
  `view()`/`on_tick()`.
- The trait implementation (`impl App for SomeStruct { ... }`) must
  stay a single, whole block — Rust does not allow splitting one
  trait's methods across multiple `impl Trait for Type` blocks. It
  always stays in `main.rs`.
- Multiple inherent `impl SomeStruct { ... }` blocks across different
  files for the same struct is ordinary, fully-supported Rust — used
  throughout Tasks 5-7.
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` clean
  after every task.
- `git mv` (not delete + recreate) for every file relocation in
  Task 4, to preserve `git log --follow` history.

---

### Task 1: Resolve `.claude/rules/development-conventions.md`

**Files:**
- Modify: `.claude/rules/development-conventions.md`

Doc-only. No TDD.

- [ ] **Step 1: Replace the file's `Status` line and open-questions
  section.** Read the current file first (it still has the "Status:
  partially defined" header and a trailing "## Open questions to
  resolve via brainstorming" section listing exactly the two items
  this plan resolves). Replace the whole file with:

```markdown
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
```

- [ ] **Step 2: Commit**

```bash
git add .claude/rules/development-conventions.md
git commit -m "$(cat <<'EOF'
docs(conventions): resolve coding style and commit conventions

Formalizes the commit format and doc-comment/file-size/directory rules
already covered in the approved design spec, closing the two items
development-conventions.md had left open since bootstrap.
EOF
)"
```

---

### Task 2: Doc comments — `src/` top-level modules

**Files:**
- Modify: `src/lib.rs`, `src/app.rs`, `src/audio.rs`, `src/buffer.rs`,
  `src/camera.rs`, `src/easing.rs`, `src/effects.rs`, `src/glitch.rs`,
  `src/layout.rs`, `src/particles.rs`, `src/terminal.rs`,
  `src/theme.rs`, `src/transition.rs`

No new tests — doc comments have no testable behavior; verified by
`cargo doc` and the `missing_docs` lint reaching zero warnings for
these files.

- [ ] **Step 1: Enable the lint** — add to the very top of `src/lib.rs`:

```rust
#![warn(missing_docs)]
```

- [ ] **Step 2: Run the lint to see the full list of gaps**

Run: `cargo clippy --lib 2>&1 | grep "missing documentation"`
Expected: one warning per undocumented `pub` item and per module
lacking a `//!` header, across every `src/` file (widgets included —
Task 3 covers those).

- [ ] **Step 3: Add a `//!` header to each of the 13 files above.**
  One to three sentences: what the module is, what it deliberately
  isn't. Example (matches this project's existing terse style, no
  filler):

```rust
//! Deterministic camera viewport and brightness scaling — used for
//! panning/zooming a rendered buffer and for boot-sequence fades.
```

  Place it as the first line(s) of each file, above any `use`
  statements.

- [ ] **Step 4: Add a single-line `///` above every `pub` item in
  these 13 files that lacks one.** Purpose and usage, not a
  restatement of the name — e.g.:

```rust
/// A 2D viewport position and zoom level over a source `Buffer`.
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

/// Crops and resamples `source` into a `width`x`height` buffer as
/// seen through `camera` — duplicates cells when `camera.zoom > 1.0`.
pub fn viewport(source: &Buffer, camera: &Camera, width: u16, height: u16) -> Buffer {
```

  Cover every `pub fn`, `pub struct`, `pub enum`, and `pub` struct
  field where the lint flags one (some, like `Camera`'s `x`/`y`/`zoom`,
  are self-evident from the struct's own doc comment plus the field
  name — the lint will tell you exactly which ones still need their
  own line).

- [ ] **Step 5: Re-run the lint filtered to these 13 files, confirm
  zero remaining gaps**

Run: `cargo clippy --lib 2>&1 | grep "missing documentation" | grep -v widgets`
Expected: empty output.

- [ ] **Step 6: `cargo test --lib` — confirm no regressions**

Expected: all existing tests still pass (doc comments don't change
behavior).

- [ ] **Step 7: `cargo fmt && cargo doc --no-deps`**

Expected: `cargo fmt` clean; `cargo doc --no-deps` builds without
error.

- [ ] **Step 8: Commit**

```bash
git add src/lib.rs src/app.rs src/audio.rs src/buffer.rs src/camera.rs src/easing.rs src/effects.rs src/glitch.rs src/layout.rs src/particles.rs src/terminal.rs src/theme.rs src/transition.rs
git commit -m "$(cat <<'EOF'
docs(core): add module and public-API doc comments

Enables #![warn(missing_docs)] and retrofits //! module headers plus
single-line /// summaries across every top-level src/ module, per the
agent-first doc-comment convention.
EOF
)"
```

---

### Task 3: Doc comments — `src/widgets/`

**Files:**
- Modify: `src/widgets/mod.rs`, `src/widgets/analog_toggle.rs`,
  `src/widgets/block.rs`, `src/widgets/damage_meter.rs`,
  `src/widgets/dial.rs`, `src/widgets/dna_console.rs`,
  `src/widgets/energy_core.rs`, `src/widgets/list.rs`,
  `src/widgets/roundel.rs`, `src/widgets/scuttle_cursor.rs`,
  `src/widgets/smash_border.rs`, `src/widgets/table.rs`,
  `src/widgets/text.rs`, `src/widgets/time_rotor.rs`

No new tests — same rationale as Task 2.

- [ ] **Step 1: Add a `//!` header to each of the 14 files above**,
  same 1-3 sentence shape as Task 2 Step 3. `mod.rs`'s header
  describes the widget module as a whole (e.g. "Ready-to-render TUI
  widgets — each takes an explicit value/theme and a target area, no
  internal animation state; the owning app computes any tweened value
  and passes a snapshot per frame.") — matches this codebase's already-
  established "dumb widget" convention.

- [ ] **Step 2: Add a single-line `///` above every `pub` item in
  these 14 files that lacks one**, same shape as Task 2 Step 4 —
  example:

```rust
/// A jerky, two-frame crab cursor: shifts left/right by one cell on
/// alternate ticks instead of gliding smoothly.
pub struct ScuttleCursor {
```

- [ ] **Step 3: Run the lint, confirm zero remaining gaps anywhere**

Run: `cargo clippy --lib 2>&1 | grep "missing documentation"`
Expected: empty output — this now covers every `src/` file (Task 2 +
Task 3 combined).

- [ ] **Step 4: `cargo test --lib`**

Expected: all tests still pass.

- [ ] **Step 5: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo doc --no-deps`**

Expected: all clean.

- [ ] **Step 6: Commit**

```bash
git add src/widgets/
git commit -m "$(cat <<'EOF'
docs(widgets): add module and public-API doc comments

Completes the doc-comment retrofit started in the previous commit —
every src/ file now has a //! header and /// summaries on its public
surface, with #![warn(missing_docs)] enforcing it going forward.
EOF
)"
```

---

### Task 5: Split `examples/omnitrix.rs` into `examples/omnitrix/`

**Files:**
- Create: `examples/omnitrix/main.rs`, `examples/omnitrix/faceplate.rs`,
  `examples/omnitrix/brainstorm.rs`, `examples/omnitrix/fasttrack.rs`,
  `examples/omnitrix/upgrade.rs`, `examples/omnitrix/boot.rs`
- Delete: `examples/omnitrix.rs`

No new tests — pure code motion, verified by running.

**Ownership map** (everything not listed stays in `main.rs`: all
`const`/`enum`/`struct` declarations per the Global Constraints, plus
`theme`, `switch_mode`, `render_mode_content` (becomes a thin
dispatcher — see Step 2), `overlay_border_noise`, `render_transition`,
`braille_noise`, `render_row`, `blit`, the whole `impl App for
Omnitrix` block, and `main()`):

| New file | Gets (as `pub(crate)` methods, `impl Omnitrix { ... }`) |
|---|---|
| `faceplate.rs` | `render_faceplate_content` (new — extracted, Step 2) |
| `brainstorm.rs` | `render_brainstorm_content` (new — extracted, Step 2) |
| `fasttrack.rs` | `render_fasttrack_content` (new — extracted, Step 2), `active_target_indices`, `render_lock_on_ring` |
| `upgrade.rs` | `render_upgrade_content` (new — extracted, Step 2), `render_circuit` |
| `boot.rs` | `render_boot` |

- [ ] **Step 1: Read the current `examples/omnitrix.rs` in full** to
  have exact line ranges and content in hand before moving anything.

- [ ] **Step 2: Extract `render_mode_content`'s four match arms into
  four new methods.** `render_mode_content` currently inlines all four
  `AppMode` variants' rendering as one 185-line `match`. Turn each arm
  into its own method taking `(&self, local: Rect, buf: &mut Buffer)`
  — keep the parameter named `local` (not `area`) so each arm's body
  moves verbatim with zero identifier renaming inside it:

```rust
fn render_mode_content(&self, mode: AppMode, area: Rect) -> Buffer {
    let mut buf = Buffer::new(area.width, area.height);
    let local = Rect {
        x: 0,
        y: 0,
        width: area.width,
        height: area.height,
    };
    match mode {
        AppMode::Faceplate => self.render_faceplate_content(local, &mut buf),
        AppMode::Brainstorm => self.render_brainstorm_content(local, &mut buf),
        AppMode::Fasttrack => self.render_fasttrack_content(local, &mut buf),
        AppMode::Upgrade => self.render_upgrade_content(local, &mut buf),
    }
    buf
}
```

  Each extracted method's signature: `pub(crate) fn render_
  faceplate_content(&self, local: Rect, buf: &mut Buffer)` (no return
  value — the original arm just wrote into `buf`), same for the other
  three, with `render_fasttrack_content`/`render_upgrade_content`
  keeping their calls to `self.active_target_indices()`/`self.
  render_lock_on_ring(...)`/`self.render_circuit(...)` unchanged
  (those move alongside into the same files — see the ownership map).
  This step stays inside `examples/omnitrix.rs` for now — the file
  split happens in Step 3.

- [ ] **Step 3: Create the five new files and the `main.rs` entry
  point**, moving each method verbatim per the ownership map above.
  Every new file (`faceplate.rs`, `brainstorm.rs`, `fasttrack.rs`,
  `upgrade.rs`, `boot.rs`) starts with:

```rust
use super::*;
```

  and contains exactly one `impl Omnitrix { ... }` block holding that
  file's methods (marked `pub(crate) fn`, since `main.rs`'s
  `render_mode_content` dispatcher and `render_transition` call into
  them across the module boundary). `examples/omnitrix/main.rs` is
  `examples/omnitrix.rs`'s remaining content (everything not in the
  ownership map) plus five `mod` declarations near the top:

```rust
mod boot;
mod brainstorm;
mod faceplate;
mod fasttrack;
mod upgrade;
```

  Delete `examples/omnitrix.rs` once `examples/omnitrix/main.rs`
  exists — Cargo resolves the `omnitrix` example target to
  `examples/omnitrix/main.rs` automatically once the flat file is
  gone (this is the same supported layout Cargo uses for multi-file
  binary targets).

- [ ] **Step 4: Build**

Run: `cargo build --example omnitrix`
Expected: compiles cleanly, no warnings. Fix any `pub(crate)`/
visibility or `use super::*` gaps the compiler flags.

- [ ] **Step 5: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean.

- [ ] **Step 6: Manual verification — re-run the ORIGINAL Omnitrix
  manual-verification checklist**

Run: `cargo run --example omnitrix`

Confirm every mode (Faceplate dial, Brainstorm, Fasttrack, Upgrade),
the boot sequence, and mode-switch transitions all behave exactly as
before the split — this is a pure code-motion, so anything different
is a bug in the extraction, not an intended change. Cross-check against
`docs/design/plans/2026-08-06-omnitrix-sub-apps-boot-arc-plan.md`'s
verification steps if anything seems off.

- [ ] **Step 7: Commit**

```bash
git add examples/omnitrix/
git rm examples/omnitrix.rs
git commit -m "$(cat <<'EOF'
refactor(omnitrix): split into a per-mode module directory

examples/omnitrix.rs had grown past the new 500-line soft ceiling
(850 lines). Pure code motion, no behavior change — verified by
re-running the app's full manual-verification checklist.
EOF
)"
```

---

### Task 6: Split `examples/tardis.rs` into `examples/tardis/`

**Files:**
- Create: `examples/tardis/main.rs`, `examples/tardis/hub.rs`,
  `examples/tardis/artron_energy.rs`, `examples/tardis/psychic_paper.rs`,
  `examples/tardis/star_charts.rs`, `examples/tardis/boot.rs`
- Delete: `examples/tardis.rs`

No new tests — pure code motion, verified by running.

**Ownership map** (everything not listed stays in `main.rs`: all
`const`/`enum`/`struct` declarations, `tardis_theme`,
`screen_for_face`, `hex_distance`, the whole `RodioAudioSink`
struct/impls, `new`, `displayed_face_index`, `time_rotor_speed`,
`is_lagging`, `render_destination_preview`, `render_transition`,
`lerp_color`, `render_ink_row`, `blit`, the whole `impl App for
Tardis` block, and `main()`). Unlike Omnitrix, Tardis's per-screen
methods are already separate (no extraction needed — pure move):

| New file | Gets (as `pub(crate)` methods, `impl Tardis { ... }`) |
|---|---|
| `hub.rs` | `render_hub`, `render_face_content` (called only from `render_hub`) |
| `artron_energy.rs` | `render_artron_energy` |
| `psychic_paper.rs` | `render_psychic_paper` |
| `star_charts.rs` | `render_star_charts` |
| `boot.rs` | `render_boot`, `render_police_box` (called only from `render_boot`) |

- [ ] **Step 1: Read the current `examples/tardis.rs` in full** to
  confirm exact method boundaries before moving anything.

- [ ] **Step 2: Create the five new files and `main.rs`**, moving each
  method verbatim per the ownership map. Same shape as Task 5 Step 3:
  each new file starts with `use super::*;`, holds one `impl Tardis {
  ... }` block with its methods marked `pub(crate) fn`, and
  `examples/tardis/main.rs` gets the remaining content plus:

```rust
mod artron_energy;
mod boot;
mod hub;
mod psychic_paper;
mod star_charts;
```

  Delete `examples/tardis.rs` once `examples/tardis/main.rs` exists.

- [ ] **Step 3: Build**

Run: `cargo build --example tardis`
Expected: compiles cleanly, no warnings.

- [ ] **Step 4: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean.

- [ ] **Step 5: Manual verification — re-run the ORIGINAL TARDIS
  manual-verification checklist**

Run: `cargo run --example tardis`

Confirm the Hub, Artron Energy, Psychic Paper, Star Charts, the boot
sequence, and camera-flight transitions all behave exactly as before.
Cross-check against `docs/design/plans/2026-08-06-tardis-console-arc-plan.md`
and `docs/design/plans/2026-08-06-tardis-remaining-sub-apps-plan.md`'s
verification steps if anything seems off.

- [ ] **Step 6: Commit**

```bash
git add examples/tardis/
git rm examples/tardis.rs
git commit -m "$(cat <<'EOF'
refactor(tardis): split into a per-screen module directory

examples/tardis.rs had grown past the new 500-line soft ceiling
(1005 lines). Pure code motion, no behavior change — verified by
re-running the app's full manual-verification checklist.
EOF
)"
```

---

### Task 7: Split `examples/smash_crabs.rs` into `examples/smash_crabs/`

**Files:**
- Create: `examples/smash_crabs/main.rs`, `examples/smash_crabs/hub.rs`,
  `examples/smash_crabs/versus.rs`, `examples/smash_crabs/target_smash.rs`,
  `examples/smash_crabs/stage_hazards.rs`, `examples/smash_crabs/boot.rs`
- Delete: `examples/smash_crabs.rs`

No new tests — pure code motion, verified by running.

**Ownership map** (everything not listed stays in `main.rs`: all
`const`/`enum`/`struct` declarations, `arena_theme`, the whole
`RodioAudioSink` struct/impls, `new`, `hub_panels`, `cursor_position`,
`displayed_p2_damage`, `shake_offset` (shared by `versus.rs` and
`target_smash.rs`), `paint_background` (shared by `versus.rs` and
`target_smash.rs`), `render_destination_preview`, `render_transition`,
`blit`, `render_row`, `render_centered_art`, `render_boot_title`, the
whole `impl App for SmashCrabs` block, and `main()`):

| New file | Gets (as `pub(crate)` methods, `impl SmashCrabs { ... }`) |
|---|---|
| `hub.rs` | `render_hub` |
| `versus.rs` | `paint_ui`, `paint_effects`, `render_versus` |
| `target_smash.rs` | `ts_visible`, `ts_smashing_is_impact`, `paint_ts_ui`, `paint_ts_effects`, `render_target_smash` |
| `stage_hazards.rs` | `sh_cpu`, `render_stage_hazards` |
| `boot.rs` | `render_boot` |

- [ ] **Step 1: Read the current `examples/smash_crabs.rs` in full**
  to confirm exact method boundaries before moving anything (it was
  last edited earlier this session — re-read rather than relying on
  memory of its content).

- [ ] **Step 2: Create the five new files and `main.rs`**, moving each
  method verbatim per the ownership map. Same shape as Task 5 Step 3:
  each new file starts with `use super::*;`, holds one `impl
  SmashCrabs { ... }` block with its methods marked `pub(crate) fn`,
  and `examples/smash_crabs/main.rs` gets the remaining content plus:

```rust
mod boot;
mod hub;
mod stage_hazards;
mod target_smash;
mod versus;
```

  Delete `examples/smash_crabs.rs` once `examples/smash_crabs/main.rs`
  exists.

- [ ] **Step 3: Build**

Run: `cargo build --example smash_crabs`
Expected: compiles cleanly, no warnings.

- [ ] **Step 4: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean.

- [ ] **Step 5: Manual verification — re-run the ORIGINAL Smash Crabs
  manual-verification checklist**

Run: `cargo run --example smash_crabs`

Confirm the boot sequence, Hub, Versus Mode, Target Smash, and Stage
Hazards all behave exactly as before. Cross-check against
`docs/design/plans/2026-08-06-smash-crabs-arena-hub-arc-plan.md` and
`docs/design/plans/2026-08-07-smash-crabs-remaining-sub-apps-plan.md`'s
verification steps if anything seems off.

- [ ] **Step 6: Commit**

```bash
git add examples/smash_crabs/
git rm examples/smash_crabs.rs
git commit -m "$(cat <<'EOF'
refactor(smash_crabs): split into a per-screen module directory

examples/smash_crabs.rs had grown past the new 500-line soft ceiling
(1069 lines). Pure code motion, no behavior change — verified by
re-running the app's full manual-verification checklist.
EOF
)"
```

---

### Task 8: `examples/README.md` index

**Files:**
- Create: `examples/README.md`

Doc-only. No TDD.

- [ ] **Step 1: Write the index**

```markdown
# Example apps

Each app is a full vertical-slice demo of the `ttui` framework, built
against a specific vision doc (`TTUI-Ideas/vision/UI/`). Run with
`cargo run --example <name>`.

- **`omnitrix`** — a dial-navigated gadget hub with three sub-apps
  (Brainstorm, Fasttrack, Upgrade) and a materialization boot sequence.
  Built from `TTUI-Ideas/vision/UI/idea-1-Omnitrix.md`.
- **`tardis`** — a hexagonal console hub with four sub-apps (Artron
  Energy, Psychic Paper, Star Charts, plus the Hub itself) and a
  camera-flight transition system. Built from
  `TTUI-Ideas/vision/UI/idea-3-TardisTUI.md`.
- **`smash_crabs`** — a character-select hub with three fighters
  (Versus Mode, Target Smash, Stage Hazards) and a Smash-Bros-style
  intro splash. Built from
  `TTUI-Ideas/vision/UI/idea-2-SuperSmashCrabs.md`.
- **`demo`** — the original core-framework smoke-test example, predates
  the vision-doc apps above. Retirement tracked in issue #83.
```

- [ ] **Step 2: Commit**

```bash
git add examples/README.md
git commit -m "docs(examples): add an index of example apps"
```

---

### Task 4: Reorganize `docs/design/` into per-Arc subdirectories

**Files:**
- Move (via `git mv`): every file currently in `docs/design/specs/`
  and `docs/design/plans/`, including the spec and this plan.
- Modify: `docs/design/README.md`

Run this task **last**, after Tasks 1-3 and 5-8 have all committed —
it moves every spec/plan file that exists by that point, including the
ones this very session's Smash Crabs and development-conventions
rounds just added.

Doc-only / `git-adjacent`. No TDD.

- [ ] **Step 1: Create the subdirectories and move specs**

```bash
mkdir -p docs/design/specs/core docs/design/specs/omnitrix docs/design/specs/tardis docs/design/specs/smash-crabs
mkdir -p docs/design/plans/core docs/design/plans/omnitrix docs/design/plans/tardis docs/design/plans/smash-crabs

git mv docs/design/specs/2026-08-04-gitops-github-workflow-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-04-testing-verification-conventions-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-04-ttui-core-framework-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-05-buffer-layering-compositing-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-05-buffer-layering-followups-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-05-claude-audit-templates-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-06-core-capabilities-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-06-example-apps-roadmap-design.md docs/design/specs/core/
git mv docs/design/specs/2026-08-07-development-conventions-design.md docs/design/specs/core/

git mv docs/design/specs/2026-08-06-omnitrix-dial-navigation-arc-design.md docs/design/specs/omnitrix/
git mv docs/design/specs/2026-08-06-omnitrix-faceplate-design.md docs/design/specs/omnitrix/
git mv docs/design/specs/2026-08-06-omnitrix-glow-border-design.md docs/design/specs/omnitrix/
git mv docs/design/specs/2026-08-06-omnitrix-sub-apps-boot-arc-design.md docs/design/specs/omnitrix/

git mv docs/design/specs/2026-08-06-tardis-console-arc-design.md docs/design/specs/tardis/
git mv docs/design/specs/2026-08-06-tardis-remaining-sub-apps-design.md docs/design/specs/tardis/

git mv docs/design/specs/2026-08-06-smash-crabs-arena-hub-arc-design.md docs/design/specs/smash-crabs/
git mv docs/design/specs/2026-08-07-smash-crabs-remaining-sub-apps-design.md docs/design/specs/smash-crabs/
```

- [ ] **Step 2: Move plans, same bucketing**

```bash
git mv docs/design/plans/2026-08-04-gitops-github-workflow-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-04-testing-verification-conventions-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-04-ttui-core-framework-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-05-buffer-layering-compositing-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-05-buffer-layering-followups-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-05-claude-audit-templates-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-05-ttui-rev-b-vision-alignment-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-06-core-capabilities-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-06-example-apps-roadmap-plan.md docs/design/plans/core/
git mv docs/design/plans/2026-08-07-development-conventions-plan.md docs/design/plans/core/

git mv docs/design/plans/2026-08-06-omnitrix-dial-navigation-arc-plan.md docs/design/plans/omnitrix/
git mv docs/design/plans/2026-08-06-omnitrix-faceplate-plan.md docs/design/plans/omnitrix/
git mv docs/design/plans/2026-08-06-omnitrix-glow-border-plan.md docs/design/plans/omnitrix/
git mv docs/design/plans/2026-08-06-omnitrix-sub-apps-boot-arc-plan.md docs/design/plans/omnitrix/

git mv docs/design/plans/2026-08-06-tardis-console-arc-plan.md docs/design/plans/tardis/
git mv docs/design/plans/2026-08-06-tardis-remaining-sub-apps-plan.md docs/design/plans/tardis/

git mv docs/design/plans/2026-08-06-smash-crabs-arena-hub-arc-plan.md docs/design/plans/smash-crabs/
git mv docs/design/plans/2026-08-07-smash-crabs-remaining-sub-apps-plan.md docs/design/plans/smash-crabs/
```

- [ ] **Step 3: Check for any spec/plan files this list missed** —
  Tasks 1-3/5-8 in this very plan only add rule-file/example changes,
  not new spec/plan files, so the two lists above should already be
  exhaustive. Confirm:

Run: `find docs/design/specs docs/design/plans -maxdepth 1 -name "*.md"`
Expected: empty (everything now lives one level deeper, under a
bucket subdirectory).

- [ ] **Step 4: Update `docs/design/README.md`** — replace:

```markdown
- `specs/` — approved design documents, one per major design decision or
  subsystem, produced by the `superpowers:brainstorming` skill. Filename
  convention: `YYYY-MM-DD-<topic>-design.md`.
- `plans/` — implementation plans derived from specs, structured as
  **Arcs → Slices → Tasks** (below), produced by the
  `superpowers:writing-plans` skill.
```

  with:

```markdown
- `specs/<arc>/` — approved design documents, one per major design
  decision or subsystem, produced by the `superpowers:brainstorming`
  skill. Filename convention: `specs/<arc>/YYYY-MM-DD-<topic>-design.md`.
- `plans/<arc>/` — implementation plans derived from specs, structured as
  **Arcs → Slices → Tasks** (below), produced by the
  `superpowers:writing-plans` skill. Same `<arc>` bucketing as `specs/`.

## Arcs

- `core/` — framework internals, tooling, process, and anything not
  scoped to one example app.
- `omnitrix/`, `tardis/`, `smash-crabs/` — one bucket per example app.
- A new bucket is added here the first time a genuinely new Arc starts
  (e.g. a fourth example app) — this list, not the file count inside
  each bucket, is what stays small as the docs tree grows.
```

- [ ] **Step 5: Confirm history survived the moves**

Run: `git log --follow --oneline docs/design/specs/core/2026-08-04-ttui-core-framework-design.md`
Expected: shows the full commit history from before the move, not just
the move commit itself.

- [ ] **Step 6: Commit**

```bash
git add docs/design/ .claude/rules/development-conventions.md
git commit -m "$(cat <<'EOF'
docs(design): reorganize specs/plans into per-Arc subdirectories

Flat directories don't stay navigable as the docs tree grows toward
the project's target scale. Moved via git mv to preserve history;
docs/design/README.md now indexes Arcs, not individual files.
EOF
)"
```

---

## Self-Review

**Spec coverage:** Slice 1 (commit conventions) and the coding-style
half of Slice 1's spec section — Task 1. Slice 2 (agent-first doc
comments, `#![warn(missing_docs)]`) — Tasks 2-3. Slice 3 (directory-
entry-count ceiling) — written into Task 1's rules-file replacement
only, no restructuring (nothing crosses the threshold yet, exactly as
the spec scoped it). Slice 4 (file-size ceiling + example splits) —
Tasks 5-7. Slice 5 (docs/ reorg) — Task 4. Slice 6 (examples index) —
Task 8. Verification section (`cargo test`/`fmt`/`clippy`/`doc`, each
split app's manual checklist, `git log --follow`, README accuracy) —
covered across every task's final steps.

**Placeholder scan:** no TBD/TODO. The doc-comment tasks (2-3) don't
pre-write every single `///` line verbatim (there are dozens across 27
files) — instead they give the exact template, two worked examples per
task, and a hard, automatic completion gate (`missing_docs` lint
reaching zero). This is deliberate: the content is fully determined by
what's already in each file (existing struct/fn names and signatures)
plus the stated template, not a judgment call being deferred — closer
to "run cargo fmt" than to "add appropriate tests." The example-app
splits (Tasks 5-7) similarly don't reproduce every moved line verbatim,
but specify exact ownership by method name and an explicit "verbatim,
no behavior change" invariant, verified by re-running each app's own
already-written manual-verification checklist.

**Type consistency:** `pub(crate)` visibility and `use super::*;` are
used identically across Tasks 5, 6, and 7. The ownership-map tables in
each of those three tasks were built from this session's own reads of
the current file structure (grep'd function/const listings plus
verified call-site checks for `render_lock_on_ring`/`render_circuit`/
`overlay_border_noise`/`render_police_box`/`render_face_content`) —
not assumed from memory.

**Task ordering:** Task 4 is placed last and explicitly says why —
it moves every file that exists in `docs/design/{specs,plans}/` by the
time it runs, including this plan and its spec, so it must run after
every other task (all of which only touch `src/`, `examples/`, and the
rules file) has already committed.
