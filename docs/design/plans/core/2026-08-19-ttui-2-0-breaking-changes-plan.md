# TTUI 2.0 Breaking Changes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `Table` per-column widths (#170), and land every other
breaking change worth making in the same major, so the next one is years
away.

**Architecture:** Seven slices, one PR each, ordered by dependency.
`#[non_exhaustive]` lands first so future `Constraint` variants are
additive. Theming lands before `Table`'s column work so `table.rs` is
opened once. The rasterizer glyph fix lands before the ellipsis so
truncated tables can be captured at all.

**Tech Stack:** Rust (stable 1.91.1, MSRV 1.87, edition 2021),
`crossterm`, new: `unicode-width`, `criterion`, `tools/visual-snapshot`.

**Design:** `docs/design/specs/core/2026-08-19-ttui-2-0-breaking-changes-design.md`

## Global Constraints

- **TDD test-first** for every `coding`-tagged task. Write the failing
  test, run it, watch it fail, then implement.
  - **Documented exception — characterisation tests.** A test that pins
    behaviour which must *not* change is written first and **passes**
    against the unmodified code; that is what makes it a
    characterisation test, and it is worthless written afterwards. Five
    are specified here and each is labelled in place: Task 1's
    non-exhaustive test (which cannot fail from inside the defining
    crate), the three `unthemed_*_keeps_the_pre_2_0_colours` tests in
    Tasks 5-7, and `without_widths_columns_split_equally` in Task 11.
    Every other test in this plan is red-first, no exceptions.
    *(Ruling by the human partner during the pre-flight scan.)*
- **Autonomy tier Gated** — a PR per slice with all four checks green:
  `cargo build --workspace`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`.
- **`semver:major`.** `CHANGELOG.md` gains a **Breaking** entry per
  breaking slice, under `## [Unreleased]`.
- **MSRV stays 1.87.** Verify with `cargo +1.87.0 check --lib` before any
  PR that adds a dependency or uses a new std API.
- **`unicode-width` is the only new dependency permitted.** It is
  `ttui`'s second ever. Do not add others.
- **Visual review is mandatory** for Slices 4 and 6 (rendering-affecting
  per `development-conventions.md`).
- **A same-code control run is required** before calling any capture
  difference real. Two runs of an identical binary differ; compare
  before/after against an after/after2 control.
- **Do not implement `Constraint::Auto`.** Explicitly deferred to a
  later minor.
- **Do not fix `Constraint::Min`'s no-growth behaviour.** Out of scope.
- Commit trailers on every commit:
  ```
  Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01RJs5Myj27GQYMS6DpUEA4b
  ```

## File Structure

| file | responsibility | slice |
|---|---|---|
| `src/buffer.rs` | `Intensity` non-exhaustive; `index` bounds check | 1, 3 |
| `src/canvas.rs` | `CanvasMode` non-exhaustive | 1 |
| `src/layout.rs` | `Direction`, `Constraint` non-exhaustive | 1 |
| `src/blend.rs` | **deleted** | 2 |
| `src/lib.rs` | drop `pub mod blend` | 2 |
| `examples/render_spike.rs` | port off `blend` | 2 |
| `benches/set.rs` | **new** — `Buffer::set` microbenchmark | 3 |
| `src/widgets/selection.rs` | **new** — shared `selection_colors` + test fixture | 4 |
| `src/widgets/mod.rs` | register `selection` | 4 |
| `src/widgets/list.rs` | `.theme()` | 4 |
| `src/widgets/dial.rs` | `.theme()` | 4 |
| `src/widgets/table.rs` | `.theme()`, then the column model | 4, 6 |
| `tools/visual-snapshot/src/glyph.rs` | U+2026 bitmap | 5 |
| `Cargo.toml` | `unicode-width`; version bump | 6, 7 |
| `examples/demo.rs` | migrate to the new `Table` API | 6 |

---

## Slice 1 — `#[non_exhaustive]` on four public enums (`coding`)

### Task 1: Mark the four enums non-exhaustive

**Files:**
- Modify: `src/buffer.rs` (`Intensity`), `src/canvas.rs` (`CanvasMode`),
  `src/layout.rs` (`Direction`, `Constraint`)
- Test: `tests/non_exhaustive.rs` (create)

**Interfaces:**
- Consumes: nothing.
- Produces: nothing new. Constrains downstream `match` only.

- [ ] **Step 1: Write the failing test**

`#[non_exhaustive]` has no effect inside the defining crate, so it
cannot be tested from a unit test in `src/`. It must be tested from
`tests/`, which compiles as a separate crate.

Create `tests/non_exhaustive.rs`:

```rust
//! `#[non_exhaustive]` only binds outside the defining crate, so this
//! lives in `tests/` rather than an inline module.

use ttui::buffer::Intensity;
use ttui::canvas::CanvasMode;
use ttui::layout::{Constraint, Direction};

// A wildcard arm is REQUIRED for each of these to compile from another
// crate. If someone removes `#[non_exhaustive]`, these still compile —
// so each is paired with a construction check below to prove the enum
// is still usable, and the real guard is the doc comment on each enum.
#[test]
fn non_exhaustive_enums_still_match_with_a_wildcard_arm() {
    let i = Intensity::Bold;
    let described = match i {
        Intensity::Normal => "normal",
        Intensity::Bold => "bold",
        Intensity::Dim => "dim",
        _ => "unknown",
    };
    assert_eq!(described, "bold");

    let m = CanvasMode::Braille;
    let described = match m {
        CanvasMode::HalfBlock => "half",
        CanvasMode::Braille => "braille",
        _ => "unknown",
    };
    assert_eq!(described, "braille");

    let d = Direction::Horizontal;
    let described = match d {
        Direction::Horizontal => "h",
        Direction::Vertical => "v",
        _ => "unknown",
    };
    assert_eq!(described, "h");

    let c = Constraint::Fill(1);
    let described = match c {
        Constraint::Fixed(_) => "fixed",
        Constraint::Percentage(_) => "pct",
        Constraint::Min(_) => "min",
        Constraint::Fill(_) => "fill",
        _ => "unknown",
    };
    assert_eq!(described, "fill");
}

#[test]
fn non_exhaustive_enums_are_still_constructible_from_another_crate() {
    // `#[non_exhaustive]` on an enum restricts exhaustive matching, not
    // variant construction. This test exists so a future reader does
    // not "fix" that by reaching for `#[non_exhaustive]` on variants,
    // which WOULD block construction and break every caller.
    let _ = Intensity::Dim;
    let _ = CanvasMode::HalfBlock;
    let _ = Direction::Vertical;
    let _ = Constraint::Percentage(50);
}
```

- [ ] **Step 2: Run the test to verify it passes BEFORE the change**

Run: `cargo test --test non_exhaustive`
Expected: PASS. This is a characterisation test — a wildcard arm is
legal with or without the attribute. It exists to prove the change does
not break construction or matching-with-wildcard.

- [ ] **Step 3: Add the attribute to all four enums**

`src/buffer.rs`, above `pub enum Intensity`:

```rust
/// Text intensity — a single SGR axis; a cell is bold, dim, or
/// neither, never more than one at once.
///
/// `#[non_exhaustive]`: new variants may be added in a minor release,
/// so downstream `match`es need a wildcard arm.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Intensity {
```

`src/canvas.rs`, above `pub enum CanvasMode` — same pattern:

```rust
#[non_exhaustive]
```

`src/layout.rs`, above `pub enum Direction` and above
`pub enum Constraint` — same pattern. `Constraint`'s doc comment gains:

```rust
/// `#[non_exhaustive]`: new variants (e.g. a content-sizing `Auto`)
/// may be added in a minor release, so downstream `match`es need a
/// wildcard arm.
```

Keep each existing `#[derive(...)]` line exactly as it is; add
`#[non_exhaustive]` above it.

- [ ] **Step 4: Verify the test still passes and nothing in-repo broke**

Run: `cargo test --workspace`
Expected: PASS, including `tests/non_exhaustive.rs`. `examples/` are
separate crates and would fail here if any matched exhaustively — a
survey found none, so a failure means a new one appeared.

- [ ] **Step 5: Changelog**

Add under `## [Unreleased]`, creating a `### Breaking` subsection above
any existing `### Added`:

```markdown
### Breaking

- `Intensity`, `CanvasMode`, `Direction` and `Constraint` are now
  `#[non_exhaustive]`. Downstream `match`es on them need a wildcard
  (`_ => …`) arm. This buys the ability to add variants — for example a
  content-sizing `Constraint::Auto` — in a *minor* release rather than
  another major.
```

- [ ] **Step 6: Four gates, then commit**

```bash
cargo build --workspace && cargo test --workspace && \
  cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
git add src/buffer.rs src/canvas.rs src/layout.rs tests/non_exhaustive.rs CHANGELOG.md
git commit -m "feat(core)!: mark the four public enums non-exhaustive"
```

- [ ] **Step 7: Open the Slice 1 PR**, stating that downstream exhaustive
      `match`es need a `_` arm and that this is what makes a future
      `Constraint::Auto` additive.

---

## Slice 2 — Retire the `blend` module (`coding`)

### Task 2: Delete `blend` and port its only consumer

**Files:**
- Delete: `src/blend.rs`
- Modify: `src/lib.rs` (drop `pub mod blend;` and its `///` line),
  `examples/render_spike.rs`

**Interfaces:**
- Consumes: `LayerStack::composite`, already public and stable.
- Produces: nothing.

- [ ] **Step 1: Read the current consumer**

Run: `grep -n "blend" examples/render_spike.rs`
Expected: `use ttui::blend::{blend_over, fade_toward};` plus call sites.
Read each call site before changing it — the port is behavioural, not
mechanical, and `render_spike` is a research spike whose output is only
checked by running it.

- [ ] **Step 2: Port `render_spike` off `blend`**

Replace `blend_over` usage with writing `Cell`s at the intended `alpha`
into separate `LayerStack` layers and letting `LayerStack::composite`
do the blending — that is the mechanism the spike recommended and the
engine adopted.

For `fade_toward`, use `easing::scale_color`, which is the shipped
equivalent (`1.0` unchanged, `0.0` black).

Keep the visual intent identical: this example exists to demonstrate
rendering fidelity levers, so a changed appearance means the port is
wrong.

- [ ] **Step 3: Delete the module**

```bash
git rm src/blend.rs
```

In `src/lib.rs`, delete both lines:

```rust
/// Alpha-blending prototype — spike prototype, not a committed API.
pub mod blend;
```

- [ ] **Step 4: Verify**

Run: `cargo build --workspace && cargo test --workspace`
Expected: PASS, with no reference to `blend` anywhere.

Run: `grep -rn "blend" src/ examples/ showcase/ --include=*.rs`
Expected: no `ttui::blend` or `mod blend` hits. Matches on the word
"blend" inside comments or `composite`'s docs are fine.

- [ ] **Step 5: Capture `render_spike` and compare**

Create `.plumb/scripts/render-spike-settle.json`:

```json
[
  { "wait_ms": 600 },
  { "wait_ms": 400 },
  { "wait_ms": 400 }
]
```

```bash
cargo build --examples
cargo run -p visual-snapshot -- --example render_spike --size 120x40 \
  --script .plumb/scripts/render-spike-settle.json --out /tmp/spike-after.gif
```

Compare against a capture from `main` **and** an after/after2 control
run. `render_spike` is a static-ish spike, so a real difference here is
a port bug, not jitter.

If the capture hard-errors on an unmapped glyph, note the codepoint and
review that region from the code instead — that is the documented
fallback in `development-conventions.md`, not a reason to skip the rest.

- [ ] **Step 6: Changelog**

```markdown
- Removed the `blend` module. Its own documentation described it as
  "spike-only, and now historical": the rendering-fidelity spike's
  recommendation was adopted, and `LayerStack::composite` has done real
  Porter-Duff "over" compositing on `Cell::alpha` ever since. Callers
  should use `LayerStack::composite` (for `blend_over`) and
  `easing::scale_color` (for `fade_toward`).
```

- [ ] **Step 7: Four gates, commit, open the Slice 2 PR**

```bash
git commit -m "refactor(core)!: retire the historical blend module"
```

---

## Slice 3 — `Buffer::set` bounds contract (#161) (`coding`)

**This slice's outcome is decided by a measurement.** Task 3 builds the
instrument; Task 4 takes the reading and acts on it.

### Task 3: Add a `Buffer::set` microbenchmark

**Files:**
- Create: `benches/set.rs`
- Modify: `Cargo.toml` (register the bench)

**Interfaces:**
- Produces: a `set` benchmark Task 4 reads. No library API change.

- [ ] **Step 1: Understand why the existing bench is not enough**

Read `benches/render.rs:98-125`. Its three profiles call `full_frame()`
/ `sparse_scatter()` / `dense_region()` **before** `b.iter()`, and the
timed closure runs only `render_diff`. `Buffer::set` is in the untimed
setup. Running it before and after a change to `index` would report no
difference regardless — do not use it as evidence.

- [ ] **Step 2: Write the benchmark**

Create `benches/set.rs`:

```rust
//! Microbenchmark for `Buffer::set`, the renderer's hottest write path.
//! Exists to decide #161: whether a real bounds check in `index` costs
//! anything measurable. `benches/render.rs` cannot answer that — it
//! builds its diffs outside the timed loop.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crossterm::style::Color;
use ttui::buffer::{Buffer, Cell};

const WIDTH: u16 = 200;
const HEIGHT: u16 = 60;

fn painted_cell(symbol: char) -> Cell {
    Cell {
        symbol,
        fg: Color::Rgb { r: 200, g: 180, b: 40 },
        bg: Color::Reset,
        alpha: 1.0,
        ..Default::default()
    }
}

fn bench_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_set");

    // A full paint pass: every cell written once, in row-major order.
    group.bench_function("full_paint", |b| {
        let mut buf = Buffer::new(WIDTH, HEIGHT);
        let cell = painted_cell('#');
        b.iter(|| {
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    buf.set(black_box(x), black_box(y), cell.clone());
                }
            }
            black_box(&buf);
        })
    });

    // A single hot cell, to isolate per-call overhead from the loop.
    group.bench_function("single_cell", |b| {
        let mut buf = Buffer::new(WIDTH, HEIGHT);
        let cell = painted_cell('*');
        b.iter(|| {
            buf.set(black_box(10), black_box(10), cell.clone());
            black_box(&buf);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_set);
criterion_main!(benches);
```

- [ ] **Step 3: Register the bench**

In `Cargo.toml`, below the existing `[[bench]]` block:

```toml
[[bench]]
name = "set"
harness = false
```

- [ ] **Step 4: Record the baseline**

Run: `cargo bench --bench set`
Expected: two reported timings. **Write both numbers into the PR
description verbatim** — they are the baseline Task 4 compares against,
and a number recalled from memory is not evidence.

- [ ] **Step 5: Commit**

```bash
git add benches/set.rs Cargo.toml
git commit -m "test(core): add a Buffer::set microbenchmark for #161"
```

### Task 4: Bounds-check `index`, gated on the measurement

**Files:**
- Modify: `src/buffer.rs` (`index`, and the `get`/`set` doc comments)
- Test: `src/buffer.rs` inline `mod tests`

**Interfaces:**
- Consumes: Task 3's benchmark.
- Produces: `Buffer::get`/`set` panic on out-of-range `x` (if Option 1
  lands). No signature change either way.

- [ ] **Step 1: Write the failing test**

Add to `src/buffer.rs`'s `mod tests`:

```rust
#[test]
#[should_panic(expected = "out of bounds")]
fn set_with_an_x_past_the_width_panics_instead_of_wrapping() {
    // Documented as a panic since 1.0, but `index` only checked the
    // flat offset: on a 4x3 buffer, set(5, 0, ..) landed on (1, 1).
    let mut buf = Buffer::new(4, 3);
    let mut cell = buf.get(0, 0).clone();
    cell.symbol = 'X';
    buf.set(5, 0, cell);
}

#[test]
#[should_panic(expected = "out of bounds")]
fn get_with_an_x_past_the_width_panics_instead_of_reading_a_neighbour() {
    let buf = Buffer::new(4, 3);
    let _ = buf.get(5, 0);
}

#[test]
fn set_at_the_last_valid_cell_still_works() {
    // Guards against an off-by-one in the new check.
    let mut buf = Buffer::new(4, 3);
    let mut cell = buf.get(0, 0).clone();
    cell.symbol = 'Z';
    buf.set(3, 2, cell);
    assert_eq!(buf.get(3, 2).symbol, 'Z');
}
```

- [ ] **Step 2: Run to verify the first two fail**

Run: `cargo test --lib buffer::tests::set_with_an_x_past`
Expected: FAIL — the test does not panic, because the write silently
lands on `(1, 1)`. That non-panic **is** #161.

- [ ] **Step 3: Implement the bounds check**

```rust
fn index(&self, x: u16, y: u16) -> usize {
    assert!(
        x < self.width && y < self.height,
        "index ({x}, {y}) out of bounds for a {}x{} buffer",
        self.width,
        self.height
    );
    y as usize * self.width as usize + x as usize
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib buffer::`
Expected: PASS, all three new tests plus every existing buffer test.

If an existing test fails, it was relying on the wrapping behaviour —
read it before changing it, because it may be documenting a real caller.

- [ ] **Step 5: Take the measurement**

Run: `cargo bench --bench set`
Compare against Task 3's recorded baseline.

- **If both profiles are within run-to-run noise** (criterion reports
  "No change in performance detected", or the change is under ~2%):
  keep the check. Update `get`/`set`'s doc comments — they already say
  "Panics if out of bounds", which finally becomes true, so add the
  precise condition:

  ```rust
  /// Returns the cell at `(x, y)`.
  ///
  /// # Panics
  /// If `x >= width` or `y >= height`.
  ```

- **If it is measurably slower**: revert the `assert!` to
  `debug_assert!`, change the two `#[should_panic]` tests to
  `#[cfg(debug_assertions)]`, and rewrite the doc comments to describe
  what release builds actually do:

  ```rust
  /// Returns the cell at `(x, y)`.
  ///
  /// # Panics
  /// In debug builds, if `x >= width` or `y >= height`. Release builds
  /// do not check; an out-of-range `x` reads a later row.
  ```

  Either way the documentation must match the code. The present state —
  a documented panic that does not happen — is the actual defect.

- [ ] **Step 6: Record the numbers**

Put the before/after timings in the PR description, and state which
option was taken and why. A future reader needs the measurement, not
the conclusion.

- [ ] **Step 7: Changelog**

For Option 1:

```markdown
- `Buffer::get`/`set` now panic on an out-of-range `x`, as they have
  always been documented to. Previously only the flat index was checked,
  so on a 4x3 buffer `set(5, 0, ..)` silently wrote to `(1, 1)` (#161).
  Code relying on that wrap will now panic — it was corrupting a
  neighbouring row.
```

For Option 2, say instead that the check is debug-only and the docs now
describe release behaviour.

- [ ] **Step 8: Four gates, commit, open the Slice 3 PR**

---

## Slice 4 — Themeable selection highlight (`coding`)

Three widgets share one hardcoded highlight. One pattern, applied three
times. **Do this before Slice 6** so `table.rs` is opened once.

### Task 5: The shared selection helper, and `List::theme`

**Ruling by the human partner during the pre-flight scan:** an earlier
draft of this plan told each widget to repeat the colour-resolution
`match` and the `test_theme()` fixture verbatim. That was overruled in
favour of one shared helper — consistent with the Shared Utilities Arc,
which spent four PRs removing exactly this shape of duplication. Tasks
6 and 7 consume the helper; they do not re-derive it.

**Files:**
- Create: `src/widgets/selection.rs`
- Modify: `src/widgets/mod.rs` (register the module),
  `src/widgets/list.rs`
- Test: `src/widgets/selection.rs` and `src/widgets/list.rs` inline
  `mod tests`

**Interfaces:**
- Produces:
  - `pub(crate) fn selection_colors(theme: Option<&Theme>, selected: bool) -> (Color, Color)`
  - `#[cfg(test)] pub(crate) fn test_theme() -> Theme` — the shared test
    fixture for Tasks 5, 6 and 7
  - `List::theme(self, theme: &'a Theme) -> Self` — a consuming builder,
    matching `Block`'s existing `.theme()` shape. Tasks 6, 7 and Slice 6
    all rely on this exact shape.

- [ ] **Step A1: Write the helper's failing test**

Create `src/widgets/selection.rs`:

```rust
//! Shared selection-highlight colour resolution for the selectable
//! widgets (`List`, `Dial`, `Table`), which each previously hardcoded
//! the same black-on-white pair and took no colours at all.

use crate::theme::Theme;
use crossterm::style::Color;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_theme_the_pre_2_0_colours_are_used() {
        assert_eq!(selection_colors(None, true), (Color::Black, Color::White));
        assert_eq!(selection_colors(None, false), (Color::Reset, Color::Reset));
    }

    #[test]
    fn with_a_theme_selection_is_accent_on_background() {
        let t = test_theme();
        assert_eq!(selection_colors(Some(&t), true), (t.accent, t.background));
    }

    #[test]
    fn with_a_theme_an_unselected_row_is_primary_on_reset() {
        let t = test_theme();
        assert_eq!(selection_colors(Some(&t), false), (t.primary, Color::Reset));
    }
}
```

- [ ] **Step A2: Run it to verify it fails**

Run: `cargo test --lib widgets::selection`
Expected: FAIL to compile — `selection_colors` and `test_theme` are not
defined, and `mod selection` is not registered.

- [ ] **Step A3: Implement the helper**

Add to `src/widgets/selection.rs`, above the test module:

```rust
/// Resolves the `(fg, bg)` pair for one row of a selectable widget.
/// Without a `Theme`, returns the fixed pre-2.0 black-on-white
/// highlight, so an untouched call site renders exactly as 1.x did.
pub(crate) fn selection_colors(theme: Option<&Theme>, selected: bool) -> (Color, Color) {
    match (theme, selected) {
        (Some(t), true) => (t.accent, t.background),
        (Some(t), false) => (t.primary, Color::Reset),
        (None, true) => (Color::Black, Color::White),
        (None, false) => (Color::Reset, Color::Reset),
    }
}
```

And the shared fixture, inside the `mod tests` block so it does not ship:

```rust
    /// Shared across `selection`, `list`, `dial` and `table` tests.
    /// Every field is set explicitly rather than via
    /// `..Default::default()`, so a fixture reader sees the whole
    /// palette the assertions depend on. (`Theme` does have a
    /// `Default` impl — src/theme.rs — this is a deliberate choice,
    /// not a necessity.)
    pub(crate) fn test_theme() -> Theme {
        use crate::theme::BorderSet;
        use crate::buffer::CellStyle;
        Theme {
            background: Color::Rgb { r: 0, g: 0, b: 32 },
            primary: Color::Rgb { r: 0, g: 255, b: 0 },
            secondary: Color::Rgb { r: 0, g: 128, b: 255 },
            tertiary: Color::Rgb { r: 255, g: 255, b: 0 },
            accent: Color::Rgb { r: 255, g: 0, b: 0 },
            primary_end: None,
            border: BorderSet::single_line(),
            border_style: CellStyle::default(),
            border_thick: false,
        }
    }
```

Check `src/theme.rs` for the current field list before writing this — if
`Theme` has gained a field, the struct literal will not compile and the
compiler will name it.

Register it in `src/widgets/mod.rs`, in the existing alphabetical run
(after `scuttle_cursor`, before `smash_border`):

```rust
/// Shared selection-highlight colour resolution.
pub(crate) mod selection;
```

Note `pub(crate)`, not `pub` — this is an internal helper, and making it
public would add API surface this Arc has not designed.

- [ ] **Step A4: Run the helper's tests**

Run: `cargo test --lib widgets::selection`
Expected: PASS, all three.

Tasks 6 and 7 reach the fixture as
`use crate::widgets::selection::tests::test_theme;`. If that path does
not resolve from another module's test config, move `test_theme` to a
`#[cfg(test)] pub(crate) mod test_support;` under `src/widgets/` and
have all four call sites use that — do not fall back to copying it.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn themed_list_uses_accent_on_background_for_the_selected_row() {
    use crate::theme::Theme;

    let items = vec!["alpha".to_string(), "beta".to_string()];
    let mut buf = Buffer::new(10, 2);
    let area = Rect { x: 0, y: 0, width: 10, height: 2 };
    let theme = Theme {
        accent: Color::Rgb { r: 255, g: 0, b: 0 },
        background: Color::Rgb { r: 0, g: 0, b: 32 },
        primary: Color::Rgb { r: 0, g: 255, b: 0 },
        ..test_theme()
    };

    List::new(&items, 1).theme(&theme).render(area, &mut buf);

    // selected row (index 1) -> accent on background
    assert_eq!(buf.get(0, 1).fg, Color::Rgb { r: 255, g: 0, b: 0 });
    assert_eq!(buf.get(0, 1).bg, Color::Rgb { r: 0, g: 0, b: 32 });
    // unselected row -> primary
    assert_eq!(buf.get(0, 0).fg, Color::Rgb { r: 0, g: 255, b: 0 });
}

#[test]
fn unthemed_list_keeps_the_pre_2_0_colours() {
    // Characterisation test: no `.theme()` must render exactly as 1.x
    // did. Worthless if written after the change.
    let items = vec!["alpha".to_string(), "beta".to_string()];
    let mut buf = Buffer::new(10, 2);
    let area = Rect { x: 0, y: 0, width: 10, height: 2 };

    List::new(&items, 1).render(area, &mut buf);

    assert_eq!(buf.get(0, 1).fg, Color::Black);
    assert_eq!(buf.get(0, 1).bg, Color::White);
    assert_eq!(buf.get(0, 0).fg, Color::Reset);
    assert_eq!(buf.get(0, 0).bg, Color::Reset);
}
```

Use the shared fixture from Step A3 rather than defining another:

```rust
    use crate::widgets::selection::tests::test_theme;
```

- [ ] **Step 2: Run to verify the first test fails**

Run: `cargo test --lib widgets::list`
Expected: FAIL to compile — "no method named `theme` found for struct
`List`". The second test passes already; that is the point of it.

- [ ] **Step 3: Implement**

```rust
pub struct List<'a> {
    items: &'a [String],
    selected: usize,
    theme: Option<&'a Theme>,
}
```

```rust
    /// Renders selection with `theme`'s `accent` on `background`, and
    /// unselected rows in `primary`. Without it, the pre-2.0 fixed
    /// black-on-white highlight is used.
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
```

In `render`, replace the hardcoded pair with the Step A3 helper:

```rust
use crate::widgets::selection::selection_colors;

let (fg, bg) = selection_colors(self.theme, i == self.selected);
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib widgets::list`
Expected: PASS, both new tests and every existing list test.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(widgets): let List take a Theme for its selection highlight"
```

### Task 6: `Dial::theme`

**Files:**
- Modify: `src/widgets/dial.rs`
- Test: `src/widgets/dial.rs` inline `mod tests`

**Interfaces:**
- Produces: `Dial::theme(self, theme: &Theme) -> Self`, identical in
  shape to `List::theme`.

- [ ] **Step 1: Write the failing test**

`Dial::new(&items, selected)` has the same constructor shape as `List`.
Its existing tests assert at `buf.get(5, 1)` for the selected item —
reuse that coordinate so the new tests exercise a cell the current suite
already trusts. Import the shared fixture rather than redefining it:

```rust
    use crate::widgets::selection::tests::test_theme;
```

```rust
#[test]
fn themed_dial_uses_accent_on_background_for_the_selected_item() {
    let items = vec!["alpha".to_string(), "beta".to_string()];
    let mut buf = Buffer::new(12, 8);
    let area = Rect { x: 0, y: 0, width: 12, height: 8 };
    let theme = test_theme();

    Dial::new(&items, 0).theme(&theme).render(area, &mut buf);

    assert_eq!(buf.get(5, 1).fg, Color::Rgb { r: 255, g: 0, b: 0 });
    assert_eq!(buf.get(5, 1).bg, Color::Rgb { r: 0, g: 0, b: 32 });
}

#[test]
fn unthemed_dial_keeps_the_pre_2_0_colours() {
    // Characterisation test: no `.theme()` must render exactly as 1.x
    // did. Worthless if written after the change.
    let items = vec!["alpha".to_string(), "beta".to_string()];
    let mut buf = Buffer::new(12, 8);
    let area = Rect { x: 0, y: 0, width: 12, height: 8 };

    Dial::new(&items, 0).render(area, &mut buf);

    assert_eq!(buf.get(5, 1).fg, Color::Black);
    assert_eq!(buf.get(5, 1).bg, Color::White);
}
```

If `(5, 1)` is not the selected cell at this area size, read
`dial.rs`'s existing tests and use whatever coordinate they assert on —
do not guess a new one.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib widgets::dial`
Expected: FAIL to compile — no method named `theme`.

- [ ] **Step 3: Implement** — add the same `Option<&'a Theme>` field and
      `.theme()` builder as Task 5, and resolve colours through
      `selection_colors(self.theme, i == self.selected)`. Do **not**
      re-derive the `match`; the helper from Task 5 Step A3 is the one
      definition.

- [ ] **Step 4: Run the tests** — `cargo test --lib widgets::dial`, PASS.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(widgets): let Dial take a Theme for its selection highlight"
```

### Task 7: `Table::theme`

**Files:**
- Modify: `src/widgets/table.rs`
- Test: `src/widgets/table.rs` inline `mod tests`

**Interfaces:**
- Produces: `Table::theme(self, theme: &Theme) -> Self`. **Slice 6
  chains onto this**, so the signature must be a consuming builder.

- [ ] **Step 1: Write the failing test**

`Table` differs from `List`/`Dial`: it has a header row as well. Pin all
three states.

```rust
#[test]
fn themed_table_colours_header_selected_and_unselected_rows() {
    let headers = vec!["Name".to_string()];
    let rows = vec![vec!["svc-a".to_string()], vec!["svc-b".to_string()]];
    let mut buf = Buffer::new(10, 3);
    let area = Rect { x: 0, y: 0, width: 10, height: 3 };
    // use crate::widgets::selection::tests::test_theme;
    let theme = test_theme();

    Table::new(&headers, &rows, 0, 5).theme(&theme).render(area, &mut buf);

    assert_eq!(buf.get(0, 0).fg, theme.secondary);          // header
    assert_eq!(buf.get(0, 1).fg, theme.accent);             // selected
    assert_eq!(buf.get(0, 1).bg, theme.background);
    assert_eq!(buf.get(0, 2).fg, theme.primary);            // unselected
}

#[test]
fn unthemed_table_keeps_the_pre_2_0_colours() {
    // ... same construction, no .theme() ...
    assert_eq!(buf.get(0, 0).fg, Color::Reset);             // header
    assert_eq!(buf.get(0, 1).fg, Color::Black);             // selected
    assert_eq!(buf.get(0, 1).bg, Color::White);
}
```

Note the header uses `secondary`, not `primary` — a header that renders
identically to its unselected rows is not a header.

- [ ] **Step 2: Run to verify it fails** — no method named `theme`.

- [ ] **Step 3: Implement.** `render_row` already takes `fg`/`bg`
      parameters, so only `render`'s two call sites choose colours; add
      the `Option<&'a Theme>` field and resolve there via
      `selection_colors(self.theme, row_idx == self.selected)`.

      The header row is the one case the helper does not cover — it is
      neither selected nor unselected. Resolve it inline:

      ```rust
      let header_fg = self.theme.map_or(Color::Reset, |t| t.secondary);
      ```

      A header that renders identically to its unselected rows is not a
      header, which is why it takes `secondary` rather than `primary`.

- [ ] **Step 4: Run the tests** — `cargo test --lib widgets::table`, PASS.

- [ ] **Step 5: Visual review**

`examples/demo.rs` is the only in-repo `Table` consumer and renders a
`List` too, so it exercises Tasks 5 and 7 together.

Create `.plumb/scripts/demo-focus-and-select.json` — `demo` binds `Tab`
to switch focus between its list and table, and `Down` to move the
selection, so this exercises both widgets' highlights:

```json
[
  { "wait_ms": 400 },
  { "key": "Down" },
  { "wait_ms": 200 },
  { "key": "Tab" },
  { "wait_ms": 200 },
  { "key": "Down" },
  { "wait_ms": 200 }
]
```

```bash
cargo build --examples
cargo run -p visual-snapshot -- --example demo --size 120x40 \
  --script .plumb/scripts/demo-focus-and-select.json --out /tmp/demo-after.gif
```

`demo` does not call `.theme()`, so **the capture must be identical to
`main` apart from jitter**. Compare before/after against an
after/after2 control run. A visible difference means a
characterisation test is missing, not that a baseline needs updating.

- [ ] **Step 6: Changelog**

```markdown
- `List`, `Dial` and `Table` accept a `Theme` via `.theme(&theme)` for
  their selection highlight; all three previously hardcoded black-on-
  white and took no colours at all. Omitting `.theme()` renders exactly
  as 1.x did.
```

- [ ] **Step 7: Four gates, commit, open the Slice 4 PR**, recording the
      demo capture and the control run in the Verification section.

---

## Slice 5 — Map U+2026 in the rasterizer (`coding`)

### Task 8: Add an ellipsis bitmap

**Files:**
- Modify: `tools/visual-snapshot/src/glyph.rs`
- Test: `tools/visual-snapshot/src/glyph.rs` inline `mod tests`

**Interfaces:**
- Produces: `glyph_for('…')` returns `Ok(_)`. Slice 6 depends on this.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn ellipsis_is_mapped_because_table_truncation_emits_it() {
    // `Table` renders U+2026 whenever a cell overflows its column,
    // which is its normal state — an unmapped ellipsis would hard-error
    // every capture of a truncated table.
    assert!(glyph_for('\u{2026}').is_ok());
}

#[test]
fn ellipsis_bitmap_is_three_dots_on_the_baseline() {
    let bitmap = glyph_for('\u{2026}').unwrap();
    // Rows 0-5 blank, row 6 carries the dots, row 7 blank.
    assert_eq!(bitmap[0], 0b0000_0000);
    assert_eq!(bitmap[5], 0b0000_0000);
    assert_ne!(bitmap[6], 0b0000_0000);
    assert_eq!(bitmap[7], 0b0000_0000);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p visual-snapshot glyph::tests::ellipsis`
Expected: FAIL — `glyph_for` returns `Err(Unmapped('…'))`. Verified: the
`BASIC`/`LATIN`/`BLOCK`/`BOX`/`MISC` font8x8 tables do not cover U+2026.

- [ ] **Step 3: Implement**

`font8x8` has no ellipsis, so supply the bitmap directly — the same
approach `braille_glyph_for` already takes for algorithmically-generated
glyphs. In `glyph_for`, before the font-table lookups:

```rust
/// U+2026 HORIZONTAL ELLIPSIS. Not in any `font8x8` table, but `Table`
/// emits it on every truncated cell, so it is supplied here rather than
/// letting captures of a normal table hard-error.
const ELLIPSIS: [u8; 8] = [
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0101_0101,
    0b0000_0000,
];

pub fn glyph_for(ch: char) -> Result<[u8; 8], GlyphError> {
    if ch == '\u{2026}' {
        return Ok(ELLIPSIS);
    }
    if let Some(bitmap) = braille_glyph_for(ch) {
        return Ok(bitmap);
    }
    // ... existing font-table chain unchanged ...
}
```

font8x8 rows are LSB-first per its existing tables; `0b0101_0101` gives
three evenly spaced dots. If the rendered ellipsis looks wrong in Step
5's capture, adjust the bit pattern — not the row index.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p visual-snapshot`
Expected: PASS, including the existing
`unmapped_glyph_is_a_hard_error_naming_the_codepoint_and_position`,
which uses U+2726 and must keep failing as before.

- [ ] **Step 5: Verify it renders**

Capture any example, then confirm visually that an ellipsis drawn
through the rasterizer reads as three dots rather than a blob or a
blank. A unit test proves it is mapped; only a capture proves it is
legible.

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(visual-snapshot): map U+2026 so truncated tables can be captured"
```

- [ ] **Step 7: Update `development-conventions.md`**

Its "Known glyph-coverage limitation" paragraph lists the current gaps.
Remove the ellipsis from any list it appears in and note that U+2026 is
now supplied directly. Leave the other listed gaps alone.

---

## Slice 6 — Table column model (#170) (`coding`)

### Task 9: Add the `unicode-width` dependency

**Files:**
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: `unicode_width::UnicodeWidthChar` / `UnicodeWidthStr` for
  Tasks 10-12.

- [ ] **Step 1: Add the dependency**

```toml
[dependencies]
crossterm = "0.27"
unicode-width = "0.2"
```

- [ ] **Step 2: Verify the MSRV still holds**

Run: `cargo +1.87.0 check --lib`
Expected: PASS. This is `ttui`'s second dependency ever; if it raises
the MSRV, stop and report rather than bumping `rust-version`, because
the MSRV is documented in the README and CONTRIBUTING.

- [ ] **Step 3: Commit**

```bash
git commit -m "build: add unicode-width for correct column measurement"
```

### Task 10: Width-aware cell truncation

**Files:**
- Modify: `src/widgets/table.rs`
- Test: `src/widgets/table.rs` inline `mod tests`

**Interfaces:**
- Produces: a private `fn fit(cell: &str, width: u16) -> String` used by
  Task 12's `render_row`. Private, so no public API change yet.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn fit_returns_short_content_untouched() {
    assert_eq!(fit("ok", 5), "ok");
}

#[test]
fn fit_truncates_with_an_ellipsis_when_content_overflows() {
    // 11 display cells of content into 6 -> 5 kept plus the marker.
    assert_eq!(fit("tardis-idle", 6), "tardi…");
}

#[test]
fn fit_into_one_cell_truncates_without_a_lone_marker() {
    // A bare "…" carries no information; prefer the first character.
    assert_eq!(fit("tardis", 1), "t");
}

#[test]
fn fit_measures_display_width_not_char_count() {
    // "東京" is 2 chars but occupies 4 cells; into 3 cells only the
    // first wide glyph plus the marker fit.
    assert_eq!(fit("東京", 3), "東…");
}

#[test]
fn fit_never_splits_a_wide_glyph_across_the_boundary() {
    // Into 2 cells, "東" alone exactly fills it; the marker would
    // overflow, so it is dropped rather than cutting the glyph.
    assert_eq!(fit("東京", 2), "東");
}

#[test]
fn fit_to_zero_width_is_empty() {
    assert_eq!(fit("anything", 0), "");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib widgets::table::tests::fit`
Expected: FAIL to compile — `fit` not found.

- [ ] **Step 3: Implement**

```rust
use unicode_width::UnicodeWidthChar;

/// Fits `cell` into `width` display cells, appending U+2026 when it is
/// cut. Measures display width rather than `char` count, so wide glyphs
/// (CJK) do not misalign the columns after them, and never splits a
/// wide glyph across the boundary.
fn fit(cell: &str, width: u16) -> String {
    let width = width as usize;
    if width == 0 {
        return String::new();
    }

    let total: usize = cell.chars().map(|c| c.width().unwrap_or(0)).sum();
    if total <= width {
        return cell.to_string();
    }

    // Room for content plus a 1-cell marker. At width 1 a lone marker
    // says nothing, so spend the cell on content instead.
    let (budget, marker) = if width > 1 { (width - 1, true) } else { (width, false) };

    let mut out = String::new();
    let mut used = 0usize;
    for c in cell.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > budget {
            break;
        }
        out.push(c);
        used += w;
    }
    if marker && used < width {
        out.push('\u{2026}');
    }
    out
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib widgets::table`
Expected: PASS, all six.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(widgets): measure table cells by display width, truncating with an ellipsis"
```

### Task 11: `widths` and `spacing` builders

**Files:**
- Modify: `src/widgets/table.rs`
- Test: `src/widgets/table.rs` inline `mod tests`

**Interfaces:**
- Produces:
  - `Table::new(headers, rows, selected) -> Table` — **`col_width` is
    removed from the signature**
  - `Table::widths(self, widths: &'a [Constraint]) -> Self`
  - `Table::spacing(self, gap: u16) -> Self`
  - private `fn column_rects(&self, area: Rect) -> Vec<Rect>`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn column_rects_gives_fixed_columns_their_width_and_fill_the_rest() {
    // #170's exact shape: narrow columns plus one that takes the rest.
    let headers = vec!["a".into(), "b".into(), "c".into()];
    let rows: Vec<Vec<String>> = vec![];
    let widths = [Constraint::Fixed(4), Constraint::Fixed(6), Constraint::Fill(1)];
    let area = Rect { x: 0, y: 0, width: 30, height: 2 };

    let t = Table::new(&headers, &rows, 0).widths(&widths);
    let rects = t.column_rects(area);

    assert_eq!(rects.len(), 3);
    assert_eq!((rects[0].x, rects[0].width), (0, 4));
    assert_eq!((rects[1].x, rects[1].width), (4, 6));
    assert_eq!((rects[2].x, rects[2].width), (10, 20)); // the rest
}

#[test]
fn spacing_inserts_a_gap_between_columns() {
    let headers = vec!["a".into(), "b".into()];
    let rows: Vec<Vec<String>> = vec![];
    let widths = [Constraint::Fixed(4), Constraint::Fixed(4)];
    let area = Rect { x: 0, y: 0, width: 20, height: 2 };

    let rects = Table::new(&headers, &rows, 0)
        .widths(&widths)
        .spacing(1)
        .column_rects(area);

    assert_eq!(rects[0].x, 0);
    assert_eq!(rects[1].x, 5); // 4 wide + 1 gap
}

#[test]
fn without_widths_columns_split_equally() {
    // Characterisation of the pre-2.0 default.
    let headers = vec!["a".into(), "b".into(), "c".into()];
    let rows: Vec<Vec<String>> = vec![];
    let area = Rect { x: 0, y: 0, width: 30, height: 2 };

    let rects = Table::new(&headers, &rows, 0).column_rects(area);

    assert_eq!(rects.len(), 3);
    for r in &rects {
        assert_eq!(r.width, 10);
    }
}

#[test]
fn more_widths_than_headers_renders_only_the_headers_columns() {
    let headers = vec!["a".into(), "b".into()];
    let rows: Vec<Vec<String>> = vec![];
    let widths = [Constraint::Fixed(3); 5];
    let area = Rect { x: 0, y: 0, width: 30, height: 2 };

    let rects = Table::new(&headers, &rows, 0).widths(&widths).column_rects(area);

    assert_eq!(rects.len(), 2);
}

#[test]
fn fewer_widths_than_headers_renders_only_the_supplied_columns() {
    let headers = vec!["a".into(), "b".into(), "c".into()];
    let rows: Vec<Vec<String>> = vec![];
    let widths = [Constraint::Fixed(3)];
    let area = Rect { x: 0, y: 0, width: 30, height: 2 };

    let rects = Table::new(&headers, &rows, 0).widths(&widths).column_rects(area);

    assert_eq!(rects.len(), 1);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib widgets::table`
Expected: FAIL to compile — `Table::new` takes 4 arguments, and there is
no `widths`, `spacing` or `column_rects`.

- [ ] **Step 3: Implement**

```rust
use crate::layout::{Constraint, Direction, Layout};

pub struct Table<'a> {
    headers: &'a [String],
    rows: &'a [Vec<String>],
    selected: usize,
    widths: Option<&'a [Constraint]>,
    spacing: u16,
    theme: Option<&'a Theme>,
}
```

```rust
    /// Creates a table over `headers`/`rows`, highlighting the data row
    /// at `selected`. Columns split equally unless [`Table::widths`] is
    /// given.
    pub fn new(headers: &'a [String], rows: &'a [Vec<String>], selected: usize) -> Self {
        Table { headers, rows, selected, widths: None, spacing: 0, theme: None }
    }

    /// Sizes each column by a [`Constraint`], the same vocabulary
    /// [`Layout`] uses — so `Fill(1)` gives a column whatever space the
    /// fixed ones leave. Columns beyond `headers.len()` are ignored,
    /// and headers beyond `widths.len()` are not rendered.
    pub fn widths(mut self, widths: &'a [Constraint]) -> Self {
        self.widths = Some(widths);
        self
    }

    /// Inserts `gap` blank cells between adjacent columns.
    pub fn spacing(mut self, gap: u16) -> Self {
        self.spacing = gap;
        self
    }

    /// One `Rect` per rendered column. `headers.len()` defines the
    /// column count; supplying fewer widths renders fewer columns.
    fn column_rects(&self, area: Rect) -> Vec<Rect> {
        let n = match self.widths {
            Some(w) => w.len().min(self.headers.len()),
            None => self.headers.len(),
        };
        let constraints: Vec<Constraint> = match self.widths {
            Some(w) => w[..n].to_vec(),
            None => vec![Constraint::Fill(1); n],
        };
        Layout::new(Direction::Horizontal, constraints)
            .spacing(self.spacing)
            .split(area)
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib widgets::table`
Expected: the five new tests PASS. **Every existing table test will fail
to compile** — they call the 4-argument `Table::new`. That includes the
three original tests *and* the two theme tests added in Task 7, which is
the one cross-slice edit in this plan.

Update them all now, so the tree stays green at every commit:

```rust
// before
Table::new(&headers, &rows, 0, 5)
// after
Table::new(&headers, &rows, 0).widths(&[Constraint::Fixed(5)])
```

For a table with more than one column, repeat the constraint per column
— `col_width: 5` across three columns becomes
`&[Constraint::Fixed(5); 3]` (`Constraint` is `Copy`, so the array
repeat syntax compiles).

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(widgets)!: give Table per-column Constraint widths and spacing"
```

### Task 12: Render through the column rects, with explicit clipping

**Files:**
- Modify: `src/widgets/table.rs`
- Test: `src/widgets/table.rs` inline `mod tests`

**Interfaces:**
- Consumes: `column_rects` (Task 11), `fit` (Task 10).
- Produces: `Table::render` honouring per-column widths.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn renders_each_cell_inside_its_own_column_rect() {
    let headers = vec!["a".into(), "b".into()];
    let rows = vec![vec!["xx".into(), "yy".into()]];
    let widths = [Constraint::Fixed(4), Constraint::Fixed(4)];
    let mut buf = Buffer::new(20, 2);
    let area = Rect { x: 0, y: 0, width: 20, height: 2 };

    Table::new(&headers, &rows, 0).widths(&widths).render(area, &mut buf);

    assert_eq!(buf.get(0, 1).symbol, 'x');
    assert_eq!(buf.get(4, 1).symbol, 'y'); // second column starts at 4
}

#[test]
fn a_column_wider_than_the_area_does_not_wrap_onto_the_next_row() {
    // Layout::split does not clamp, and Buffer::set wraps an
    // out-of-range x onto a later row (#161). Table must clip itself.
    let headers = vec!["a".into()];
    let rows = vec![vec!["abcdefghij".into()]];
    let widths = [Constraint::Fixed(50)]; // far wider than the area
    let mut buf = Buffer::new(4, 3);
    let area = Rect { x: 0, y: 0, width: 4, height: 3 };

    Table::new(&headers, &rows, 0).widths(&widths).render(area, &mut buf);

    // Row 2 was never written to.
    for x in 0..4 {
        assert_eq!(buf.get(x, 2).symbol, ' ');
    }
}

#[test]
fn a_wide_glyph_cell_leaves_the_next_column_aligned() {
    let headers = vec!["a".into(), "b".into()];
    let rows = vec![vec!["東京".into(), "ok".into()]];
    let widths = [Constraint::Fixed(4), Constraint::Fixed(4)];
    let mut buf = Buffer::new(20, 2);
    let area = Rect { x: 0, y: 0, width: 20, height: 2 };

    Table::new(&headers, &rows, 0).widths(&widths).render(area, &mut buf);

    assert_eq!(buf.get(4, 1).symbol, 'o'); // still starts at 4
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib widgets::table`
Expected: FAIL — cells still render at the old fixed stride.

- [ ] **Step 3: Implement**

Rewrite `render_row` to take the column rects:

```rust
    fn render_row(
        &self,
        rects: &[Rect],
        area: Rect,
        y: u16,
        cells: &[String],
        fg: Color,
        bg: Color,
        buf: &mut Buffer,
    ) {
        for (rect, cell) in rects.iter().zip(cells) {
            let text = fit(cell, rect.width);
            let mut x = rect.x;
            for ch in text.chars() {
                // Explicit clip: Layout::split does not clamp, and
                // Buffer::set wraps an out-of-range x onto a later row.
                if x >= area.x + area.width || x >= rect.x + rect.width {
                    break;
                }
                buf.set(x, y, Cell { symbol: ch, fg, bg, alpha: 1.0, ..Default::default() });
                x += ch.width().unwrap_or(1) as u16;
            }
        }
    }
```

`render` computes `let rects = self.column_rects(area);` once and passes
it to both the header call and each row call.

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib widgets::table`
Expected: PASS, every table test including Tasks 10 and 11's.

- [ ] **Step 5: Migrate `examples/demo.rs`**

```rust
// before
Table::new(&self.table_headers, &self.table_rows, self.table_selected, 12)

// after
Table::new(&self.table_headers, &self.table_rows, self.table_selected)
    .widths(&[Constraint::Fill(1), Constraint::Fill(1)])
```

`demo` has two columns and previously used one width for both, so two
equal `Fill(1)`s reproduce it. Add `Constraint` to its `ttui::layout`
import.

- [ ] **Step 6: Visual review**

```bash
cargo build --examples
cargo run -p visual-snapshot -- --example demo --size 120x40 \
  --script .plumb/scripts/demo-focus-and-select.json \
  --out /tmp/demo-t12-after.gif
```

The migration is intended to be output-identical: equal `Fill(1)`
columns across the same area reproduce the old equal split. Compare
before/after against an after/after2 control. **A real difference means
the migration is wrong** — re-derive it rather than accepting a new
baseline.

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(widgets)!: render Table cells through per-column rects"
```

### Task 13: Changelog and Slice 6 PR

- [ ] **Step 1: Changelog**

```markdown
- **`Table::new` no longer takes `col_width`.** Columns are sized with
  `Constraint`s — the same vocabulary `Layout` uses — via
  `.widths(&[...])`, so a table can mix narrow fixed columns with one
  that takes the remaining width (#170). Without `.widths()`, columns
  split equally, as `col_width` effectively did. `.spacing(n)` inserts
  a gap between columns, and cells that overflow their column are cut
  with an ellipsis instead of silently ending. Cells are measured by
  display width, so CJK and combining marks no longer misalign the
  columns after them.

  ```rust
  // 1.x
  Table::new(&headers, &rows, selected, 12).render(area, buf);
  // 2.0
  Table::new(&headers, &rows, selected)
      .widths(&[Constraint::Fixed(6), Constraint::Fill(1)])
      .render(area, buf);
  ```
```

- [ ] **Step 2: Four gates, then open the Slice 6 PR**, recording the
      `demo` captures, the control run, and the new dependency.

---

## Slice 7 — Cut 2.0.0 (`admin`)

### Task 14: Verify the release

- [ ] **Step 1: Confirm the API surface changed as intended**

```bash
git diff v1.1.0 --stat -- src/
```

Then diff the public surface the same way the v1.1.0 release did:
extract every `^\s*pub (fn|struct|enum|trait|const|type|mod)` line under
`src/` at `v1.1.0` and at `HEAD`, sort, and compare. Expected removals:
`pub mod blend` and its two functions. Expected signature change:
`Table::new`. Anything else removed is unintended — investigate before
tagging.

- [ ] **Step 2: Run all five `.plumb` scenarios and read every frame**

Exit code 0 is not evidence. Note that `tardis-console-idle` renders no
widgets at all (#165), so it does not cover Slices 4 or 6 despite
declaring `touches: src/widgets/**`.

- [ ] **Step 3: Re-verify the MSRV**

```bash
cargo +1.87.0 check --lib && cargo +1.87.0 test --doc
```

- [ ] **Step 4: Check the rustdoc landing page still compiles**

`src/lib.rs`'s quick-start doctest uses `Text`, not `Table`, so it
should be unaffected — but `cargo test --doc` must pass, and if any
example in a doc comment used `blend` or the old `Table::new`, fix it.

### Task 15: Bump and hand off

**Files:** `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`

- [ ] **Step 1: Bump `version` to `2.0.0`**, then `cargo check` to sync
      `Cargo.lock`.

- [ ] **Step 2: Convert `[Unreleased]` to `## [2.0.0] - <date>`**, and
      **lead the section with a migration table** — a consumer reads
      that first:

  | 1.x | 2.0 |
  |---|---|
  | `Table::new(h, r, s, w)` | `Table::new(h, r, s).widths(&[…])` |
  | `blend::blend_over` | `LayerStack::composite` |
  | `blend::fade_toward` | `easing::scale_color` |
  | exhaustive `match` on `Constraint` etc. | add a `_ => …` arm |

- [ ] **Step 3: `cargo publish --dry-run`**

- [ ] **Step 4: Update `README.md`'s Status section** to 2.0.0 and
      replace the "v1.1 is additive" paragraph with what 2.0 changed.

- [ ] **Step 5: STOP — Human tier.** Tagging `v2.0.0`, pushing the tag,
      cutting the GitHub release and `cargo publish` are the user's to
      run: publishing is irreversible and needs their credentials.
      Report the exact commands rather than running them.

---

## Notes for the implementer

- **`parallax-panopticon` is the reason this Arc exists.** After 2.0.0
  publishes, it needs updating for `Table::new` and possibly a `_` arm
  on any `Constraint` match. That is a separate repository and outside
  this plan, but it is the acceptance test for whether #170 is actually
  fixed.
- **Slice 3's outcome is not predetermined.** If the benchmark says the
  bounds check costs real time, taking `debug_assert!` is the correct
  outcome, not a failure. Record the numbers either way.
- **Do not tick these checkboxes expecting them to be read as progress.**
  No plan in this repository has ever had them ticked; the PRs are the
  record. Tick them if you like, but the four green checks and the PR
  description are what the next person will actually trust.
