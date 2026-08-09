# Rendering Primitives Graduation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Graduate three of the rendering-fidelity spike's six levers into real, TDD-covered, committed core API: a `CellStyle.intensity: Intensity` enum (replacing `bold: bool`) plus the four boolean style attributes, the `Canvas` sub-cell primitive, and a gradient-border option on `Theme`/`Block`.

**Architecture:** `Intensity` is a new enum in `src/buffer.rs`; `Canvas` is promoted in place from `src/canvas.rs` (same file, same public signatures, now with full tests); `Theme` gains one new `Option<Color>` field consumed by `Block::render`'s existing border-drawing closures. No new files, no new public function signatures beyond what the spike already prototyped.

**Tech Stack:** Rust, `crossterm` 0.27, existing `ttui` core (`buffer`, `terminal`, `theme`, `easing`, `layout`).

## Global Constraints

- **Tag: `coding`. Full TDD applies to every task — no exceptions.** Every new/changed behavior gets a failing test first, per `.claude/rules/development-conventions.md`. This is the opposite posture from the spike this Arc graduates from.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are **hard gates** on every task — not the spike's relaxed posture.
- `Theme.border_bold: bool` (a *different*, pre-existing field, meaning "should Block draw border cells bold") is **not** touched or renamed by this plan — only `CellStyle.bold` (the field that field maps *into*) changes shape. Do not confuse the two.
- One worktree for this whole Arc, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- `coding`-tagged → **Gated** autonomy tier: ships as a PR to `main` with all four required checks green, squash-merged at the end.
- Spec being implemented: `docs/design/specs/core/2026-08-08-rendering-primitives-graduation-design.md`.

---

### Task 1: `Intensity` enum + `CellStyle` field migration

**Files:**
- Modify: `src/buffer.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub enum Intensity { Normal, Bold, Dim }`, `CellStyle.intensity: Intensity` (replaces `CellStyle.bold: bool`) — every later task in this plan constructs/reads this field.

- [ ] **Step 1: Write the failing tests**

In `src/buffer.rs`'s existing `#[cfg(test)] mod tests`, replace:

```rust
    #[test]
    fn cell_style_default_bold_is_false() {
        assert!(!CellStyle::default().bold);
    }
```

```rust
    #[test]
    fn cells_identical_except_bold_are_unequal() {
        let cell1 = Cell::default();
        let mut cell2 = Cell::default();
        cell2.style.bold = true;
        assert_ne!(cell1, cell2);
    }
```

with:

```rust
    #[test]
    fn intensity_default_is_normal() {
        assert_eq!(Intensity::default(), Intensity::Normal);
    }

    #[test]
    fn cell_style_default_intensity_is_normal() {
        assert_eq!(CellStyle::default().intensity, Intensity::Normal);
    }

    #[test]
    fn cells_identical_except_intensity_are_unequal() {
        let cell1 = Cell::default();
        let mut cell2 = Cell::default();
        cell2.style.intensity = Intensity::Bold;
        assert_ne!(cell1, cell2);
    }

    #[test]
    fn bold_dim_and_normal_are_pairwise_distinct() {
        assert_ne!(Intensity::Normal, Intensity::Bold);
        assert_ne!(Intensity::Bold, Intensity::Dim);
        assert_ne!(Intensity::Normal, Intensity::Dim);
    }
```

Leave `cell_default_style_equals_cell_style_default` unchanged — it doesn't reference `bold` directly.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib buffer::tests`
Expected: FAIL to compile — `Intensity` doesn't exist yet, `CellStyle` has no `intensity` field.

- [ ] **Step 3: Implement `Intensity` and migrate `CellStyle`**

Change:

```rust
/// Text styling flags for a single `Cell`, beyond color.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CellStyle {
    /// Whether the cell renders bold.
    pub bold: bool,
    /// Whether the cell renders underlined.
    pub underline: bool,
    /// Whether the cell renders italic.
    pub italic: bool,
    /// Whether fg/bg render swapped.
    pub reverse: bool,
    /// Whether the cell renders with a strikethrough.
    pub strikethrough: bool,
}
```

to:

```rust
/// Text intensity — a single SGR axis; a cell is bold, dim, or
/// neither, never more than one at once.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Intensity {
    /// No intensity styling.
    #[default]
    Normal,
    /// Bold (increased intensity).
    Bold,
    /// Dim (decreased intensity).
    Dim,
}

/// Text styling flags for a single `Cell`, beyond color.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CellStyle {
    /// Bold/dim/neither — mutually exclusive by construction.
    pub intensity: Intensity,
    /// Whether the cell renders underlined.
    pub underline: bool,
    /// Whether the cell renders italic.
    pub italic: bool,
    /// Whether fg/bg render swapped.
    pub reverse: bool,
    /// Whether the cell renders with a strikethrough.
    pub strikethrough: bool,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib buffer::tests`
Expected: PASS (this task's own tests). The crate as a whole will NOT compile yet — every other file constructing `CellStyle { bold: ... }` or reading `.style.bold` is now broken. That's expected; Tasks 2-3 fix each site. Do not attempt to fix other files in this task.

- [ ] **Step 5: Commit**

```bash
git add src/buffer.rs
git commit -m "feat(core): replace CellStyle.bold with a three-state Intensity enum

Bold and dim are one SGR intensity axis, not independent flags — this
is the cheapest point to fix that shape, before a second bolted-on
dim: bool field ever ships. Downstream call sites are migrated in
follow-up commits on this branch."
```

---

### Task 2: `render_diff` intensity wiring + coalescing tests

**Files:**
- Modify: `src/terminal.rs`

**Interfaces:**
- Consumes: `Intensity` (Task 1).
- Produces: nothing new downstream — this is a leaf consumer of `CellStyle.intensity`.

- [ ] **Step 1: Write the failing tests**

In `src/terminal.rs`'s `render_diff_tests` module, change the `d()` helper's signature and every call site's last argument, then add one new test. First, the helper:

```rust
    fn d(x: u16, y: u16, symbol: char, fg: Color, bg: Color, intensity: Intensity) -> CellDiff {
        CellDiff {
            x,
            y,
            cell: Cell {
                symbol,
                fg,
                bg,
                style: CellStyle {
                    intensity,
                    ..Default::default()
                },
            },
        }
    }
```

Update the `use` line above it from `use crate::buffer::{Cell, CellDiff, CellStyle};` to `use crate::buffer::{Cell, CellDiff, CellStyle, Intensity};`.

Update every existing `d(...)` call's last argument: `false` → `Intensity::Normal`, `true` → `Intensity::Bold`, across all six existing tests (`single_diff_emits_move_colors_intensity_and_glyph`, `contiguous_same_styled_run_moves_once_and_sets_style_once`, `positional_gap_forces_a_second_move`, `new_row_forces_a_second_move`, `color_change_mid_run_re_emits_that_color_only`, `bold_toggle_emits_intensity_transitions`) — twelve call sites total, one `false`→`Intensity::Normal` or `true`→`Intensity::Bold` substitution per call, no other change to any of these six tests.

Add a new test after `bold_toggle_emits_intensity_transitions`:

```rust
    #[test]
    fn intensity_cycles_through_all_three_states() {
        let out = render(&[
            d(3, 2, 'A', Color::Reset, Color::Reset, Intensity::Normal),
            d(4, 2, 'B', Color::Reset, Color::Reset, Intensity::Bold),
            d(5, 2, 'C', Color::Reset, Color::Reset, Intensity::Dim),
            d(6, 2, 'D', Color::Reset, Color::Reset, Intensity::Normal),
        ]);
        assert_eq!(
            count(&out, &encode(SetAttribute(Attribute::NormalIntensity))),
            2,
            "first cell (Normal) + final Normal transition"
        );
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Bold))), 1);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Dim))), 1);
    }
```

The spike shipped `render_diff`'s underline/italic/reverse/strikethrough wiring with **zero tests** (by design, per the spike's research-tag exemption) — this task is where that gap actually closes. `d()`'s signature only carries `intensity`, not the four booleans, so these new tests build `CellDiff`s directly rather than through `d()`:

```rust
    fn styled(x: u16, y: u16, symbol: char, style: CellStyle) -> CellDiff {
        CellDiff {
            x,
            y,
            cell: Cell {
                symbol,
                fg: Color::Reset,
                bg: Color::Reset,
                style,
            },
        }
    }

    #[test]
    fn underline_toggle_emits_underlined_and_no_underline() {
        let out = render(&[
            styled(3, 2, 'A', CellStyle::default()),
            styled(
                4,
                2,
                'B',
                CellStyle {
                    underline: true,
                    ..Default::default()
                },
            ),
            styled(5, 2, 'C', CellStyle::default()),
        ]);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Underlined))), 1);
        assert_eq!(
            count(&out, &encode(SetAttribute(Attribute::NoUnderline))),
            2,
            "first cell + underline-off transition"
        );
    }

    #[test]
    fn italic_toggle_emits_italic_and_no_italic() {
        let out = render(&[
            styled(3, 2, 'A', CellStyle::default()),
            styled(
                4,
                2,
                'B',
                CellStyle {
                    italic: true,
                    ..Default::default()
                },
            ),
            styled(5, 2, 'C', CellStyle::default()),
        ]);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Italic))), 1);
        assert_eq!(
            count(&out, &encode(SetAttribute(Attribute::NoItalic))),
            2,
            "first cell + italic-off transition"
        );
    }

    #[test]
    fn reverse_toggle_emits_reverse_and_no_reverse() {
        let out = render(&[
            styled(3, 2, 'A', CellStyle::default()),
            styled(
                4,
                2,
                'B',
                CellStyle {
                    reverse: true,
                    ..Default::default()
                },
            ),
            styled(5, 2, 'C', CellStyle::default()),
        ]);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Reverse))), 1);
        assert_eq!(
            count(&out, &encode(SetAttribute(Attribute::NoReverse))),
            2,
            "first cell + reverse-off transition"
        );
    }

    #[test]
    fn strikethrough_toggle_emits_crossed_out_and_not_crossed_out() {
        let out = render(&[
            styled(3, 2, 'A', CellStyle::default()),
            styled(
                4,
                2,
                'B',
                CellStyle {
                    strikethrough: true,
                    ..Default::default()
                },
            ),
            styled(5, 2, 'C', CellStyle::default()),
        ]);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::CrossedOut))), 1);
        assert_eq!(
            count(&out, &encode(SetAttribute(Attribute::NotCrossedOut))),
            2,
            "first cell + strikethrough-off transition"
        );
    }

    #[test]
    fn all_five_style_axes_combine_independently_in_one_cell() {
        let out = render(&[styled(
            3,
            2,
            'A',
            CellStyle {
                intensity: Intensity::Bold,
                underline: true,
                italic: true,
                reverse: true,
                strikethrough: true,
            },
        )]);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Bold))), 1);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Underlined))), 1);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Italic))), 1);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::Reverse))), 1);
        assert_eq!(count(&out, &encode(SetAttribute(Attribute::CrossedOut))), 1);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib terminal::render_diff_tests`
Expected: FAIL to compile — `render_diff` still reads `d.cell.style.bold`, which no longer exists, and the underline/italic/reverse/strikethrough assertions above fail against the current (already-shipped, spike-era) wiring is not the point here since that wiring already exists unchanged from the spike; the actual failure is the compile error from `d.cell.style.bold`. Once Step 3 fixes that, these five new tests should already pass against the spike's existing (untested until now) attribute wiring — they exist to close the coverage gap, not to change behavior.

- [ ] **Step 3: Implement the three-way intensity wiring**

In `render_diff`, change:

```rust
    let mut last_bold: Option<bool> = None;
```

to:

```rust
    let mut last_intensity: Option<Intensity> = None;
```

and change:

```rust
        // NormalIntensity (not a full SGR reset) clears bold without
        // touching color, so fg/bg can be tracked independently.
        let bold = d.cell.style.bold;
        if last_bold != Some(bold) {
            let attr = if bold {
                Attribute::Bold
            } else {
                Attribute::NormalIntensity
            };
            queue!(writer, SetAttribute(attr))?;
            last_bold = Some(bold);
        }
```

to:

```rust
        // NormalIntensity (not a full SGR reset) clears bold/dim without
        // touching color, so fg/bg can be tracked independently.
        let intensity = d.cell.style.intensity;
        if last_intensity != Some(intensity) {
            let attr = match intensity {
                Intensity::Normal => Attribute::NormalIntensity,
                Intensity::Bold => Attribute::Bold,
                Intensity::Dim => Attribute::Dim,
            };
            queue!(writer, SetAttribute(attr))?;
            last_intensity = Some(intensity);
        }
```

Add `Intensity` to the top-of-file import: change `use crate::buffer::CellDiff;` to `use crate::buffer::{CellDiff, Intensity};`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib terminal::render_diff_tests`
Expected: PASS, all 7 tests (6 migrated + 1 new).

- [ ] **Step 5: Commit**

```bash
git add src/terminal.rs
git commit -m "feat(core): wire render_diff for three-state Intensity"
```

---

### Task 3: Migrate remaining `CellStyle { bold: ... }` call sites

**Files:**
- Modify: `src/widgets/block.rs`
- Modify: `benches/render.rs`
- Modify: `examples/launcher/main.rs`
- Modify: `examples/smash_crabs/smash_crabs.rs`
- Modify: `examples/smash_crabs/target_smash.rs`
- Modify: `examples/render_spike.rs`

**Interfaces:**
- Consumes: `Intensity` (Task 1).
- Produces: a compiling workspace — this task's exit criterion is `cargo build --all-targets` succeeding for the first time since Task 1 landed.

This task is `coding`-tagged but every change is a mechanical field-rename at an already-tested call site (block.rs's render logic and its own tests are exercised by Task 6, not here) — no new test-first cycle applies to a pure rename; the existing tests **at each site** are the regression check. Do not skip re-running them.

- [ ] **Step 1: `src/widgets/block.rs` — construction site**

Change:

```rust
        let plain = || Cell {
            symbol: ' ',
            fg,
            bg,
            style: CellStyle {
                bold: border_bold,
                ..Default::default()
            },
        };
```

to:

```rust
        let plain = || Cell {
            symbol: ' ',
            fg,
            bg,
            style: CellStyle {
                intensity: if border_bold {
                    Intensity::Bold
                } else {
                    Intensity::Normal
                },
                ..Default::default()
            },
        };
```

Add `Intensity` to the top-of-file import: change `use crate::buffer::{Buffer, Cell, CellStyle};` to `use crate::buffer::{Buffer, Cell, CellStyle, Intensity};`.

- [ ] **Step 2: `src/widgets/block.rs` — test assertions**

In the `tests` module, change `use crate::buffer::Buffer;` to `use crate::buffer::{Buffer, Intensity};`. Then change these five assertions:

`without_theme_border_colors_are_reset`: `assert!(!buf.get(0, 0).style.bold);` → `assert_eq!(buf.get(0, 0).style.intensity, Intensity::Normal);`

`border_cells_are_bold_when_theme_border_bold_is_true` (all three lines): `assert!(buf.get(0, 0).style.bold);` / `assert!(buf.get(1, 0).style.bold);` / `assert!(buf.get(0, 1).style.bold);` → `assert_eq!(buf.get(0, 0).style.intensity, Intensity::Bold);` / `assert_eq!(buf.get(1, 0).style.intensity, Intensity::Bold);` / `assert_eq!(buf.get(0, 1).style.intensity, Intensity::Bold);`

`title_cells_are_not_bold_even_when_theme_border_bold_is_true` (both lines): `assert!(!buf.get(1, 0).style.bold);` / `assert!(!buf.get(2, 0).style.bold);` → `assert_eq!(buf.get(1, 0).style.intensity, Intensity::Normal);` / `assert_eq!(buf.get(2, 0).style.intensity, Intensity::Normal);`

- [ ] **Step 3: `benches/render.rs`**

Change:

```rust
        let attr = if d.cell.style.bold {
            Attribute::Bold
        } else {
            Attribute::Reset
        };
```

to:

```rust
        let attr = if d.cell.style.intensity == ttui::buffer::Intensity::Bold {
            Attribute::Bold
        } else {
            Attribute::Reset
        };
```

and:

```rust
fn themed(symbol: char) -> Cell {
    Cell {
        symbol,
        fg: Color::Green,
        bg: Color::Reset,
        style: CellStyle {
            bold: false,
            ..Default::default()
        },
    }
}
```

to:

```rust
fn themed(symbol: char) -> Cell {
    Cell {
        symbol,
        fg: Color::Green,
        bg: Color::Reset,
        style: CellStyle {
            intensity: ttui::buffer::Intensity::Normal,
            ..Default::default()
        },
    }
}
```

(Fully-qualifying `ttui::buffer::Intensity` here matches this file's existing style of fully-qualifying `ttui::buffer::*`/`ttui::terminal::*` rather than adding a new `use` line, consistent with how `diff`/`Buffer`/`Cell`/`CellDiff`/`CellStyle` are already imported via one `use ttui::buffer::{...}` line — add `Intensity` to that existing import list instead if you prefer; either is fine, pick one and use it consistently in this file.)

- [ ] **Step 4: `examples/launcher/main.rs`**

Change:

```rust
                style: CellStyle {
                    bold,
                    ..Default::default()
                },
```

(inside `text_center`, where `bold: bool` is a function parameter — the parameter itself is unchanged, only its use in the `CellStyle` literal) to:

```rust
                style: CellStyle {
                    intensity: if bold {
                        ttui::buffer::Intensity::Bold
                    } else {
                        ttui::buffer::Intensity::Normal
                    },
                    ..Default::default()
                },
```

(Same fully-qualification note as Step 3 applies — use whichever import style this file already uses elsewhere.)

- [ ] **Step 5: `examples/smash_crabs/smash_crabs.rs`**

Change:

```rust
                        style: CellStyle {
                            bold: true,
                            ..Default::default()
                        },
```

(the "VS" label cell) to:

```rust
                        style: CellStyle {
                            intensity: ttui::buffer::Intensity::Bold,
                            ..Default::default()
                        },
```

- [ ] **Step 6: `examples/smash_crabs/target_smash.rs`**

Change:

```rust
                            style: CellStyle {
                                bold: true,
                                ..Default::default()
                            },
```

(the KO-stamp cell) to:

```rust
                            style: CellStyle {
                                intensity: ttui::buffer::Intensity::Bold,
                                ..Default::default()
                            },
```

- [ ] **Step 7: `examples/render_spike.rs`**

Change (inside `draw_gradient_ring`'s `ring_cell` closure):

```rust
        Cell {
            symbol,
            fg: hue_to_rgb(t * 180.0 + hue_shift),
            bg: Color::Reset,
            style: CellStyle {
                bold: true,
                ..Default::default()
            },
        }
```

to:

```rust
        Cell {
            symbol,
            fg: hue_to_rgb(t * 180.0 + hue_shift),
            bg: Color::Reset,
            style: CellStyle {
                intensity: ttui::buffer::Intensity::Bold,
                ..Default::default()
            },
        }
```

- [ ] **Step 8: Build and test the whole workspace**

Run: `cargo build --all-targets`
Expected: succeeds — this is the first point since Task 1 the whole workspace compiles.

Run: `cargo test`
Expected: full suite passes, including every test touched in Steps 1-2.

- [ ] **Step 9: Commit**

```bash
git add src/widgets/block.rs benches/render.rs examples/launcher/main.rs \
  examples/smash_crabs/smash_crabs.rs examples/smash_crabs/target_smash.rs \
  examples/render_spike.rs
git commit -m "fix(core): migrate remaining CellStyle.bold call sites to Intensity"
```

---

### Task 4: `Canvas` — full test suite + committed-API polish

**Files:**
- Modify: `src/canvas.rs`

**Interfaces:**
- Consumes: `Canvas`, `CanvasMode` (unchanged public signatures from the spike).
- Produces: nothing new downstream in this plan — Task 6/a future Arc B are the first real consumers.

- [ ] **Step 1: Update the module doc comment**

Change:

```rust
//! Sub-cell rendering primitive (half-block + braille) — SPIKE
//! PROTOTYPE for the rendering-fidelity spike
//! (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md).
//! Not a committed, stable API: expect this to be rewritten once the
//! spike's recommendations are acted on.
```

to:

```rust
//! Sub-cell rendering primitive: `HalfBlock` mode gives 2x vertical
//! resolution with full 2-color fidelity per cell; `Braille` mode
//! gives 4x resolution with one fg color per cell. Graduated from the
//! rendering-fidelity spike
//! (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md)
//! per
//! docs/design/specs/core/2026-08-08-rendering-primitives-graduation-design.md.
```

- [ ] **Step 2: `saturating_add` fix in `rect`/`fill_rect`**

Change:

```rust
    pub fn rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        if w == 0 || h == 0 {
            return;
        }
        self.line(x, y, x + w - 1, y, color);
        self.line(x, y + h - 1, x + w - 1, y + h - 1, color);
        self.line(x, y, x, y + h - 1, color);
        self.line(x + w - 1, y, x + w - 1, y + h - 1, color);
    }
```

to:

```rust
    pub fn rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        if w == 0 || h == 0 {
            return;
        }
        let x1 = x.saturating_add(w).saturating_sub(1);
        let y1 = y.saturating_add(h).saturating_sub(1);
        self.line(x, y, x1, y, color);
        self.line(x, y1, x1, y1, color);
        self.line(x, y, x, y1, color);
        self.line(x1, y, x1, y1, color);
    }
```

and change:

```rust
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        for row in y..y + h {
            for col in x..x + w {
                self.set_pixel(col, row, color);
            }
        }
    }
```

to:

```rust
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        for row in y..y.saturating_add(h) {
            for col in x..x.saturating_add(w) {
                self.set_pixel(col, row, color);
            }
        }
    }
```

- [ ] **Step 3: Write the failing tests**

Add a `#[cfg(test)] mod tests` block at the end of `src/canvas.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Cell;

    fn red() -> Color {
        Color::Rgb { r: 255, g: 0, b: 0 }
    }
    fn blue() -> Color {
        Color::Rgb { r: 0, g: 0, b: 255 }
    }

    #[test]
    fn half_block_top_only_produces_upper_half_block_with_reset_bg() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 0, red());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: '▀',
                fg: red(),
                bg: Color::Reset,
                ..Default::default()
            }
        );
    }

    #[test]
    fn half_block_bottom_only_produces_lower_half_block_with_reset_bg() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 1, blue());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: '▄',
                fg: blue(),
                bg: Color::Reset,
                ..Default::default()
            }
        );
    }

    #[test]
    fn half_block_both_equal_produces_solid_block() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 0, red());
        c.set_pixel(0, 1, red());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: '█',
                fg: red(),
                bg: red(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn half_block_both_different_splits_fg_and_bg() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 0, red());
        c.set_pixel(0, 1, blue());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: '▀',
                fg: red(),
                bg: blue(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn half_block_unset_cell_leaves_target_buffer_untouched() {
        let c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        let mut buf = Buffer::new(1, 1);
        let sentinel = Cell {
            symbol: 'X',
            fg: Color::Green,
            bg: Color::Yellow,
            ..Default::default()
        };
        buf.set(0, 0, sentinel.clone());
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), sentinel);
    }

    #[test]
    fn set_pixel_out_of_bounds_is_silently_ignored() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(99, 99, red()); // grid is only 1x2 for a 1x1 half-block canvas
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn clear_pixel_removes_a_previously_set_pixel() {
        let mut c = Canvas::new(1, 1, CanvasMode::HalfBlock);
        c.set_pixel(0, 0, red());
        c.clear_pixel(0, 0);
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn braille_single_dot_produces_the_correct_glyph_and_color() {
        let mut c = Canvas::new(1, 1, CanvasMode::Braille);
        c.set_pixel(0, 0, red()); // top-left dot: bit 0x01
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        let expected_symbol = char::from_u32(0x2800 + 0x01).unwrap();
        assert_eq!(
            *buf.get(0, 0),
            Cell {
                symbol: expected_symbol,
                fg: red(),
                bg: Color::Reset,
                ..Default::default()
            }
        );
    }

    #[test]
    fn braille_two_dots_combine_their_bits_into_one_glyph() {
        let mut c = Canvas::new(1, 1, CanvasMode::Braille);
        c.set_pixel(0, 0, red()); // bit 0x01
        c.set_pixel(1, 0, red()); // bit 0x08
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        let expected_symbol = char::from_u32(0x2800 + 0x01 + 0x08).unwrap();
        assert_eq!(buf.get(0, 0).symbol, expected_symbol);
    }

    #[test]
    fn braille_last_written_dot_wins_the_cells_color() {
        let mut c = Canvas::new(1, 1, CanvasMode::Braille);
        c.set_pixel(0, 0, red());
        c.set_pixel(1, 0, blue());
        let mut buf = Buffer::new(1, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(buf.get(0, 0).fg, blue());
    }

    #[test]
    fn braille_unset_cell_leaves_target_buffer_untouched() {
        let c = Canvas::new(1, 1, CanvasMode::Braille);
        let mut buf = Buffer::new(1, 1);
        let sentinel = Cell {
            symbol: 'X',
            fg: Color::Green,
            bg: Color::Yellow,
            ..Default::default()
        };
        buf.set(0, 0, sentinel.clone());
        c.blit(&mut buf, 0, 0);
        assert_eq!(*buf.get(0, 0), sentinel);
    }

    #[test]
    fn line_draws_a_horizontal_run_of_pixels() {
        let mut c = Canvas::new(3, 1, CanvasMode::HalfBlock);
        c.line(0, 0, 2, 0, red()); // top row of a 3-wide, 1-tall (2-subpixel-tall) canvas
        let mut buf = Buffer::new(3, 1);
        c.blit(&mut buf, 0, 0);
        for x in 0..3 {
            assert_eq!(buf.get(x, 0).symbol, '▀', "cell {x} should show the top-only half-block");
            assert_eq!(buf.get(x, 0).fg, red());
        }
    }

    #[test]
    fn rect_draws_all_four_edges_of_the_outline() {
        let mut c = Canvas::new(3, 3, CanvasMode::HalfBlock); // grid 3x6
        c.rect(0, 0, 3, 6, red());
        let mut buf = Buffer::new(3, 3);
        c.blit(&mut buf, 0, 0);
        // Every perimeter cell should have picked up red on at least one subpixel;
        // the center cell (1,1) should remain untouched (default).
        assert_ne!(*buf.get(0, 0), Cell::default());
        assert_ne!(*buf.get(1, 0), Cell::default());
        assert_ne!(*buf.get(2, 0), Cell::default());
        assert_ne!(*buf.get(0, 1), Cell::default());
        assert_ne!(*buf.get(2, 1), Cell::default());
        assert_ne!(*buf.get(0, 2), Cell::default());
        assert_ne!(*buf.get(1, 2), Cell::default());
        assert_ne!(*buf.get(2, 2), Cell::default());
        assert_eq!(*buf.get(1, 1), Cell::default());
    }

    #[test]
    fn fill_rect_fills_every_pixel_in_the_region() {
        let mut c = Canvas::new(2, 1, CanvasMode::HalfBlock); // grid 2x2
        c.fill_rect(0, 0, 2, 2, red());
        let mut buf = Buffer::new(2, 1);
        c.blit(&mut buf, 0, 0);
        assert_eq!(buf.get(0, 0).symbol, '█');
        assert_eq!(buf.get(1, 0).symbol, '█');
    }

    #[test]
    fn blit_clips_to_the_target_buffers_bounds_without_panicking() {
        let mut c = Canvas::new(3, 3, CanvasMode::HalfBlock);
        c.fill_rect(0, 0, 3, 6, red());
        let mut buf = Buffer::new(2, 2); // smaller than the canvas
        c.blit(&mut buf, 0, 0); // must not panic
        assert_eq!(buf.get(0, 0).symbol, '█');
        assert_eq!(buf.get(1, 1).symbol, '█');
    }

    #[test]
    fn rect_and_fill_rect_near_u16_max_do_not_panic() {
        let mut c = Canvas::new(4, 4, CanvasMode::HalfBlock);
        c.rect(u16::MAX - 2, u16::MAX - 2, 10, 10, red()); // must not panic on overflow
        c.fill_rect(u16::MAX - 2, u16::MAX - 2, 10, 10, red()); // must not panic on overflow
    }
}
```

- [ ] **Step 4: Run tests to verify they fail, then pass**

Run: `cargo test --lib canvas::tests`
Expected: with `Step 1-2`'s code already in place before writing tests would normally invert red/green — for this task, since `Canvas`'s logic is unchanged from the spike except the `saturating_add` fix, write the tests first per the checklist above, confirm they compile against the *current* (spike) code and pass for everything except the two new `rect`/`fill_rect` overflow-safety tests, THEN apply Step 2's fix, then re-run to confirm all pass. If you already applied Step 2 before writing tests, that's fine too — just ensure the final `cargo test --lib canvas::tests` run shows all tests passing, and that `rect_and_fill_rect_near_u16_max_do_not_panic` would have failed (panicked) against the pre-Step-2 code, confirmed by temporarily reverting Step 2 and re-running just that one test if you want the RED evidence explicitly.

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean (hard gates now, per Global Constraints).

- [ ] **Step 6: Commit**

```bash
git add src/canvas.rs
git commit -m "feat(core): graduate Canvas to committed API with full test coverage

Adds the test suite the spike deliberately shipped without, and fixes
rect/fill_rect's bounds arithmetic to saturate instead of panicking
near u16::MAX now that this is committed code."
```

---

### Task 5: `Theme.primary_end` field + exhaustive-literal fixups

**Files:**
- Modify: `src/theme.rs`
- Modify: `src/widgets/smash_border.rs`
- Modify: `src/widgets/block.rs` (five `Theme { ... }` test literals)
- Modify: `examples/tardis/tardis.rs`
- Modify: `examples/launcher/portal.rs`
- Modify: `examples/omnitrix/omnitrix.rs`
- Modify: `examples/smash_crabs/smash_crabs.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `Theme.primary_end: Option<Color>` — Task 6 reads this field.

`Theme` is `#[derive(Clone, Copy, Debug, PartialEq)]` with no `Default` shortcut used at any of its 9 construction sites in the codebase (all are fully exhaustive literals) — adding a field breaks every one of them, same exhaustiveness rule as `CellStyle`. Every site below already has an `accent:`/`accent,` field; insert the new field immediately after it.

- [ ] **Step 1: Write the failing tests**

In `src/theme.rs`'s `tests` module, add:

```rust
    #[test]
    fn default_theme_primary_end_is_none() {
        assert_eq!(Theme::default().primary_end, None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib theme::tests`
Expected: FAIL to compile — `Theme` has no `primary_end` field yet.

- [ ] **Step 3: Add the field**

In `src/theme.rs`, change:

```rust
pub struct Theme {
    /// Base background color.
    pub background: Color,
    /// Main accent/brand color.
    pub primary: Color,
    /// Secondary accent color.
    pub secondary: Color,
    /// Tertiary accent color.
    pub tertiary: Color,
    /// Highlight/selection color.
    pub accent: Color,
    /// Border glyph set.
    pub border: BorderSet,
    /// Whether borders render bold.
    pub border_bold: bool,
    /// Whether `Block` draws an outward second border ring.
    pub border_thick: bool,
}
```

to:

```rust
pub struct Theme {
    /// Base background color.
    pub background: Color,
    /// Main accent/brand color.
    pub primary: Color,
    /// Secondary accent color.
    pub secondary: Color,
    /// Tertiary accent color.
    pub tertiary: Color,
    /// Highlight/selection color.
    pub accent: Color,
    /// When set, `Block` lerps the border ring's color from `primary`
    /// to this across the ring's perimeter instead of a flat color.
    pub primary_end: Option<Color>,
    /// Border glyph set.
    pub border: BorderSet,
    /// Whether borders render bold.
    pub border_bold: bool,
    /// Whether `Block` draws an outward second border ring.
    pub border_thick: bool,
}
```

and in `impl Default for Theme`, add `primary_end: None,` immediately after `accent: Color::Reset,`.

- [ ] **Step 4: Fix the eight remaining exhaustive `Theme { ... }` literals**

In each file below, insert `primary_end: None,` immediately after the existing `accent:`/`accent,` line:

- `src/widgets/smash_border.rs` — `test_theme()`, after `accent: Color::Yellow,`.
- `src/widgets/block.rs` — five separate `Theme { ... }` literals in the `tests` module (`with_theme_border_uses_theme_glyphs_and_colors`, `border_cells_are_bold_when_theme_border_bold_is_true`, `thick_border_draws_a_second_ring_one_cell_outward`, `thin_border_leaves_the_outward_ring_untouched`, `title_cells_are_not_bold_even_when_theme_border_bold_is_true`), each after their `accent: Color::Reset,` line.
- `examples/tardis/tardis.rs` — `tardis_theme()`, after its `accent:` line.
- `examples/launcher/portal.rs` — after its `accent,` line (shorthand field, not `accent: accent,`).
- `examples/omnitrix/omnitrix.rs` — `theme()`, after its `accent: Color::White,` line.
- `examples/smash_crabs/smash_crabs.rs` — `arena_theme()`, after its `accent: Color::Yellow,` line.

Read each file's actual current content before editing — do not guess at exact surrounding formatting; insert the one line in the right place per file's real layout.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo build --all-targets && cargo test`
Expected: workspace compiles, full test suite passes (including the new test from Step 1).

- [ ] **Step 6: Commit**

```bash
git add src/theme.rs src/widgets/smash_border.rs src/widgets/block.rs \
  examples/tardis/tardis.rs examples/launcher/portal.rs \
  examples/omnitrix/omnitrix.rs examples/smash_crabs/smash_crabs.rs
git commit -m "feat(core): add Theme.primary_end for optional gradient borders"
```

---

### Task 6: Gradient-border rendering in `Block::render`

**Files:**
- Modify: `src/widgets/block.rs`

**Interfaces:**
- Consumes: `Theme.primary_end` (Task 5), `easing::lerp_color` (existing, unchanged).
- Produces: nothing new downstream in this plan.

- [ ] **Step 1: Write the failing tests**

Add to `src/widgets/block.rs`'s `tests` module:

```rust
    #[test]
    fn primary_end_none_produces_byte_for_byte_identical_output_to_flat_color() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '*',
            },
            border_bold: false,
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        // Every border cell must be flat theme.primary — the exact
        // regression guarantee for existing themed apps that never
        // set primary_end.
        for x in 0..4 {
            assert_eq!(buf.get(x, 0).fg, Color::Green);
            assert_eq!(buf.get(x, 2).fg, Color::Green);
        }
        for y in 0..3 {
            assert_eq!(buf.get(0, y).fg, Color::Green);
            assert_eq!(buf.get(3, y).fg, Color::Green);
        }
    }

    #[test]
    fn primary_end_some_lerps_color_across_the_border_ring() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Rgb { r: 0, g: 0, b: 0 },
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: Some(Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            }),
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '*',
            },
            border_bold: false,
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        // Top-left corner (0,0) is at perimeter position t=0 -> exactly primary.
        assert_eq!(buf.get(0, 0).fg, Color::Rgb { r: 0, g: 0, b: 0 });
        // Bottom-right corner (3,2) is at perimeter position t=1 (clamped) -> exactly primary_end.
        assert_eq!(
            buf.get(3, 2).fg,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50
            }
        );
        // A cell strictly between the two corners must differ from both endpoints.
        let mid = buf.get(3, 0).fg;
        assert_ne!(mid, Color::Rgb { r: 0, g: 0, b: 0 });
        assert_ne!(
            mid,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50
            }
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::block::tests`
Expected: FAIL — `Theme` literals in the new tests won't compile until Task 5 has landed (it has, by this point in the plan), and `primary_end_some_lerps_color_across_the_border_ring` fails its assertions since `Block::render` doesn't read `primary_end` yet.

- [ ] **Step 3: Implement gradient-ring rendering**

Replace the theme-resolution and `plain`/`draw_ring` closures with:

```rust
        let (border, fg, bg, border_bold, border_thick, primary_end) = match self.theme {
            Some(t) => (
                t.border,
                t.primary,
                t.background,
                t.border_bold,
                t.border_thick,
                t.primary_end,
            ),
            None => (BorderSet::default(), Color::Reset, Color::Reset, false, false, None),
        };
        let ring_fg = |x: u16, y: u16| -> Color {
            match primary_end {
                Some(end) => {
                    let t = ((x as f32 - area.x as f32) / area.width.max(1) as f32
                        + (y as f32 - area.y as f32) / area.height.max(1) as f32)
                        .clamp(0.0, 1.0);
                    crate::easing::lerp_color(fg, end, t)
                }
                None => fg,
            }
        };
        let plain = |x: u16, y: u16| Cell {
            symbol: ' ',
            fg: ring_fg(x, y),
            bg,
            style: CellStyle {
                intensity: if border_bold {
                    Intensity::Bold
                } else {
                    Intensity::Normal
                },
                ..Default::default()
            },
        };

        let draw_ring = |ring_area: Rect, buf: &mut Buffer| {
            for x in ring_area.x..ring_area.x + ring_area.width {
                buf.set(
                    x,
                    ring_area.y,
                    Cell {
                        symbol: border.horizontal,
                        ..plain(x, ring_area.y)
                    },
                );
                buf.set(
                    x,
                    ring_area.y + ring_area.height - 1,
                    Cell {
                        symbol: border.horizontal,
                        ..plain(x, ring_area.y + ring_area.height - 1)
                    },
                );
            }
            for y in ring_area.y..ring_area.y + ring_area.height {
                buf.set(
                    ring_area.x,
                    y,
                    Cell {
                        symbol: border.vertical,
                        ..plain(ring_area.x, y)
                    },
                );
                buf.set(
                    ring_area.x + ring_area.width - 1,
                    y,
                    Cell {
                        symbol: border.vertical,
                        ..plain(ring_area.x + ring_area.width - 1, y)
                    },
                );
            }
            buf.set(
                ring_area.x,
                ring_area.y,
                Cell {
                    symbol: border.corner,
                    ..plain(ring_area.x, ring_area.y)
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                ring_area.y,
                Cell {
                    symbol: border.corner,
                    ..plain(ring_area.x + ring_area.width - 1, ring_area.y)
                },
            );
            buf.set(
                ring_area.x,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.corner,
                    ..plain(ring_area.x, ring_area.y + ring_area.height - 1)
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.corner,
                    ..plain(
                        ring_area.x + ring_area.width - 1,
                        ring_area.y + ring_area.height - 1
                    )
                },
            );
        };
```

And in the title-drawing block, change:

```rust
        if let Some(title) = self.title {
            for (i, ch) in title
                .chars()
                .take(area.width.saturating_sub(2) as usize)
                .enumerate()
            {
                buf.set(
                    area.x + 1 + i as u16,
                    area.y,
                    Cell {
                        symbol: ch,
                        style: CellStyle::default(),
                        ..plain()
                    },
                );
            }
        }
```

to:

```rust
        if let Some(title) = self.title {
            for (i, ch) in title
                .chars()
                .take(area.width.saturating_sub(2) as usize)
                .enumerate()
            {
                let x = area.x + 1 + i as u16;
                buf.set(
                    x,
                    area.y,
                    Cell {
                        symbol: ch,
                        style: CellStyle::default(),
                        ..plain(x, area.y)
                    },
                );
            }
        }
```

Add `Intensity` to the top-of-file import: change `use crate::buffer::{Buffer, Cell, CellStyle};` to `use crate::buffer::{Buffer, Cell, CellStyle, Intensity};`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::block::tests`
Expected: PASS, all tests including the two new ones and every pre-existing test (`draws_border_and_returns_inner_area`, `title_is_drawn_on_the_top_border`, etc.) — the pre-existing tests never set `primary_end`, so `Theme::default()`/explicit-`None`-equivalent construction means `ring_fg` degenerates to the flat `fg` they already assert on.

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 6: Commit**

```bash
git add src/widgets/block.rs
git commit -m "feat(core): gradient border rendering when Theme.primary_end is set"
```

---

### Task 7: Final workspace verification

**Files:** none (verification only).

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: full suite green, including every test added/changed across Tasks 1-6.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds — library, all examples, benches.

- [ ] **Step 4: Manual visual regression check**

Run: `cargo run --example omnitrix`, `cargo run --example tardis`, `cargo run --example smash_crabs` in turn.
Expected: every border that was bold before this Arc (via `Theme.border_bold`) still renders bold — `Intensity::Bold` is a drop-in replacement for `bold: true`, and no example sets `primary_end` yet, so every border stays flat-colored exactly as before. No visual regression anywhere. Press `q` to quit each.

- [ ] **Step 5: Commit (if Step 4 required any fix) or proceed**

If Step 4 surfaces no issues, there is nothing to commit for this task — it's a verification gate, not a code change.

---

## Final verification (whole plan)

- [ ] `cargo test` — full suite green.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean (hard gates, unlike the spike this Arc graduates from).
- [ ] `cargo build --all-targets` — library, examples, benches all compile.
- [ ] Manual visual check on Omnitrix/TARDIS/Smash Crabs confirms zero regression — every prior `bold` usage still renders bold, no theme accidentally shows a gradient it didn't opt into.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree`.
