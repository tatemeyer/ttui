# Pre-v1 Fix Wave Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close GitHub issues #108, #115, #119 — the exact 3 findings
the sweep audit triaged as `v1-blocking` — by generalizing
`Theme.border_bold` to `Theme.border_style: CellStyle`, adding an
additive `GlitchBuffer::with_alpha` builder, and replacing
`Camera::project_line`'s 8-argument signature with a
`ProjectLineParams` struct.

**Architecture:** Three independent slices (different subsystems, no
shared code, no ordering dependency), each closing exactly one issue.
Task boundaries match the slices 1:1.

**Tech Stack:** Rust.

## Global Constraints

- **`coding`-tagged, TDD mandatory** — no exemption for any task.
- **Scope is exactly issues #108, #115, #119.** No other API surface
  changes, no `project_polygon` changes, no broader `Theme` redesign.
- **`GlitchBuffer::with_alpha` must be purely additive** — existing
  callers (`GlitchBuffer::new()` with no `.with_alpha()` call) must
  render identically to today (`alpha: 1.0`).
- **Visual review is mandatory** for Task 1 (touches `src/widgets/`)
  and Task 2 (touches `src/glitch.rs`) per
  `.claude/rules/development-conventions.md` — run `tools/visual-
  snapshot` against an affected example and read the resulting PNG/GIF
  before marking either task done.
- **Close each issue** by referencing it in that task's commit message
  (e.g. `Closes #108`).

---

### Task 1: `Theme.border_style: CellStyle`

**Files:**
- Modify: `src/theme.rs`
- Modify: `src/widgets/block.rs`
- Modify: `src/widgets/cockpit_panel.rs`
- Modify: `src/widgets/smash_border.rs`
- Modify: `examples/control_panel.rs`
- Modify: `examples/falcon/falcon.rs`
- Modify: `examples/launcher/portal.rs`
- Modify: `examples/mission_control.rs`
- Modify: `examples/omnitrix/omnitrix.rs`
- Modify: `examples/smash_crabs/smash_crabs.rs`
- Modify: `examples/tardis/tardis.rs`

**Interfaces:**
- Produces: `Theme.border_style: CellStyle` (replaces
  `Theme.border_bold: bool`) — no other task consumes this.

- [ ] **Step 1: Update the failing tests first**

In `src/theme.rs`, replace:

```rust
    #[test]
    fn default_theme_border_bold_is_false() {
        assert!(!Theme::default().border_bold);
    }
```

with:

```rust
    #[test]
    fn default_theme_border_style_is_default() {
        assert_eq!(Theme::default().border_style, CellStyle::default());
    }
```

(add `use crate::buffer::CellStyle;` to `src/theme.rs`'s existing
`use` block if not already present — check first, it likely isn't
since `Theme` currently has no `CellStyle` field.)

In `src/widgets/block.rs`'s test module, in
`border_cells_are_bold_when_theme_border_bold_is_true`, replace:

```rust
            border_bold: true,
```

with:

```rust
            border_style: CellStyle {
                intensity: Intensity::Bold,
                ..Default::default()
            },
```

Rename the test itself to `border_cells_are_bold_when_theme_border_style_is_bold`
(its body/assertions are unchanged — only the theme construction and
name change).

In `title_cells_are_not_bold_even_when_theme_border_bold_is_true`,
replace:

```rust
            border_bold: true,
```

with the same `border_style: CellStyle { intensity: Intensity::Bold, ..Default::default() }`
block. Rename the test to
`title_cells_are_not_bold_even_when_theme_border_style_is_bold` (body
unchanged).

In the remaining 6 `Theme { .. }` literals in `block.rs`'s test module
(`with_theme_border_uses_theme_glyphs_and_colors`,
`thick_border_draws_a_second_ring_one_cell_outward`,
`thin_border_leaves_the_outward_ring_untouched`,
`primary_end_none_produces_byte_for_byte_identical_output_to_flat_color`,
`primary_end_some_lerps_color_across_the_border_ring`,
`non_rgb_primary_with_primary_end_renders_flat_end_color_not_a_gradient`),
each currently has `border_bold: false,` — replace every one with
`border_style: CellStyle::default(),`. (`CellStyle` is already
imported in `block.rs` via `use crate::buffer::{Buffer, Cell, CellStyle, Intensity};`
— confirm this import line already covers it before editing; it does
per the file's current top-of-file imports.)

In `src/widgets/cockpit_panel.rs`, replace:

```rust
            border_bold: false,
```

with:

```rust
            border_style: CellStyle::default(),
```

(add `CellStyle` to this file's existing `use crate::buffer::{...}`
import line if not already present — check first.)

In `src/widgets/smash_border.rs`, same replacement:
`border_bold: false,` → `border_style: CellStyle::default(),` (check
and add the `CellStyle` import if needed).

In `examples/control_panel.rs`, `examples/mission_control.rs`, and
`examples/tardis/tardis.rs` — each has exactly one
`border_bold: false,` — replace with `border_style: CellStyle::default(),`.
Each of these 3 files' `use ttui::buffer::{...};` line needs `CellStyle`
added (confirmed via grep: none of the three currently import it).

In `examples/falcon/falcon.rs` — same replacement
(`border_bold: false,` → `border_style: CellStyle::default(),`); its
`use ttui::buffer::{Cell, LayerStack};` line needs `CellStyle` added
too (becomes `use ttui::buffer::{Cell, CellStyle, LayerStack};`).

In `examples/smash_crabs/smash_crabs.rs` — same replacement; this
file's import line already includes `CellStyle` and `Intensity`
(`use ttui::buffer::{Buffer, Cell, CellStyle, Intensity, LayerStack};`)
— no import change needed here.

In `examples/launcher/portal.rs`, replace:

```rust
        border_bold: focused && pulse > 0.5,
```

with:

```rust
        border_style: CellStyle {
            intensity: if focused && pulse > 0.5 {
                Intensity::Bold
            } else {
                Intensity::Normal
            },
            ..Default::default()
        },
```

Add `CellStyle, Intensity` to this file's
`use ttui::buffer::Buffer;` line — it currently imports only `Buffer`
from `ttui::buffer`, so it becomes
`use ttui::buffer::{Buffer, CellStyle, Intensity};`.

In `examples/omnitrix/omnitrix.rs`, replace:

```rust
            border_bold: brightness > 0.6,
```

with:

```rust
            border_style: CellStyle {
                intensity: if brightness > 0.6 {
                    Intensity::Bold
                } else {
                    Intensity::Normal
                },
                ..Default::default()
            },
```

This file's existing import is
`use ttui::buffer::{Buffer, Cell, LayerStack};` — add `CellStyle` and
`Intensity`, becoming
`use ttui::buffer::{Buffer, Cell, CellStyle, Intensity, LayerStack};`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib theme:: widgets::block::`
Expected: FAIL to compile — `Theme`/`Block`'s test module references
`border_style`, which doesn't exist on `Theme` yet (still has
`border_bold`).

- [ ] **Step 3: Change `Theme`'s field**

In `src/theme.rs`, replace:

```rust
    /// Whether borders render bold.
    pub border_bold: bool,
```

with:

```rust
    /// Style applied to every border cell (not title cells — see
    /// `Block::render`). Reuses `Cell`'s own style type rather than a
    /// narrower bool, so future border attributes (underline, etc.)
    /// need no further `Theme` field growth.
    pub border_style: CellStyle,
```

In `Theme::default()`, replace `border_bold: false,` with
`border_style: CellStyle::default(),`.

- [ ] **Step 4: Update `Block::render` to consume `border_style`**

In `src/widgets/block.rs`, replace:

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
            None => (
                BorderSet::default(),
                Color::Reset,
                Color::Reset,
                false,
                false,
                None,
            ),
        };
```

with:

```rust
        let (border, fg, bg, border_style, border_thick, primary_end) = match self.theme {
            Some(t) => (
                t.border,
                t.primary,
                t.background,
                t.border_style,
                t.border_thick,
                t.primary_end,
            ),
            None => (
                BorderSet::default(),
                Color::Reset,
                Color::Reset,
                CellStyle::default(),
                false,
                None,
            ),
        };
```

Then replace:

```rust
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
            alpha: 1.0,
        };
```

with:

```rust
        let plain = |x: u16, y: u16| Cell {
            symbol: ' ',
            fg: ring_fg(x, y),
            bg,
            style: border_style,
            alpha: 1.0,
        };
```

- [ ] **Step 5: Apply all the call-site changes from Step 1**

(These were written as part of Step 1's failing-test-first pass for
the test files; now apply the same `border_bold` → `border_style`
replacements to the 9 **non-test** call sites: `cockpit_panel.rs`,
`smash_border.rs`, and the 7 example files. Each replacement and each
necessary import addition is fully specified in Step 1 above — do them
now.)

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --lib theme:: widgets::`
Expected: all tests pass, including the 2 renamed tests
(`border_cells_are_bold_when_theme_border_style_is_bold`,
`title_cells_are_not_bold_even_when_theme_border_style_is_bold`) and
`default_theme_border_style_is_default`.

- [ ] **Step 7: Build everything (catches the example files)**

Run: `cargo build --all-targets`
Expected: succeeds — this is what actually catches any missed
`border_bold` reference or missing import in the 7 example files,
since `cargo test --lib` alone doesn't compile examples.

- [ ] **Step 8: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 9: Full workspace test**

Run: `cargo test --workspace`
Expected: full suite green, no regressions elsewhere.

- [ ] **Step 10: Mandatory visual review**

Run `tools/visual-snapshot` against an example whose theme sets
`border_style` to bold under some condition — `omnitrix` is the best
choice (its `border_bold: brightness > 0.6` condition, now
`border_style`'s conditional `Intensity::Bold`, is easy to trigger by
scripting enough ticks/interactions to raise `brightness` past 0.6).
Capture at least one frame and `Read` it, confirming the border
renders bold exactly as before this change. Note the capture in this
task's completion record.

- [ ] **Step 11: Commit**

```bash
git add src/theme.rs src/widgets/block.rs src/widgets/cockpit_panel.rs \
        src/widgets/smash_border.rs examples/control_panel.rs \
        examples/falcon/falcon.rs examples/launcher/portal.rs \
        examples/mission_control.rs examples/omnitrix/omnitrix.rs \
        examples/smash_crabs/smash_crabs.rs examples/tardis/tardis.rs
git commit -m "feat(core): generalize Theme.border_bold to border_style: CellStyle

Reuses Cell's existing CellStyle type instead of a narrower bool, so
future border attributes need no further Theme field growth. Free now
(pre-1.0); every Theme { .. } literal in the repo is exhaustive, so
this would be a breaking change once ttui hits 1.0.

Closes #108"
```

---

### Task 2: `GlitchBuffer::with_alpha`

**Files:**
- Modify: `src/glitch.rs`

**Interfaces:**
- Produces: `GlitchBuffer::with_alpha(self, alpha: f32) -> Self` — not
  consumed by any other task in this plan (Falcon/TARDIS adoption, if
  wanted, is a separate future change — this task only adds the
  capability).

- [ ] **Step 1: Write the failing tests**

Add to `src/glitch.rs`'s existing `#[cfg(test)] mod tests` block,
after `at_full_intensity_every_cell_is_glitched_with_the_requested_color`:

```rust
    #[test]
    fn default_alpha_is_1_0() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(500));
        let mut buf = Buffer::new(3, 3);

        gb.render(area(), Color::Red, 0, &mut buf);

        assert_eq!(buf.get(1, 1).alpha, 1.0);
    }

    #[test]
    fn with_alpha_sets_every_rendered_cells_alpha() {
        let mut gb = GlitchBuffer::new().with_alpha(0.5);
        gb.trigger(Duration::from_millis(500));
        let mut buf = Buffer::new(3, 3);

        gb.render(area(), Color::Red, 0, &mut buf);

        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(buf.get(x, y).alpha, 0.5);
            }
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib glitch::`
Expected: FAIL to compile — `GlitchBuffer::with_alpha` doesn't exist
yet.

- [ ] **Step 3: Add the `alpha` field and `with_alpha` builder**

In `src/glitch.rs`, replace:

```rust
pub struct GlitchBuffer {
    transition: Option<Transition>,
}

impl GlitchBuffer {
    /// Creates an inactive `GlitchBuffer`.
    pub fn new() -> Self {
        GlitchBuffer { transition: None }
    }
```

with:

```rust
pub struct GlitchBuffer {
    transition: Option<Transition>,
    alpha: f32,
}

impl GlitchBuffer {
    /// Creates an inactive `GlitchBuffer`.
    pub fn new() -> Self {
        GlitchBuffer {
            transition: None,
            alpha: 1.0,
        }
    }

    /// Sets the alpha every rendered glitch cell carries, for a
    /// partially-transparent effect (e.g. "static laid over the
    /// readout, not fully opaque"). Defaults to `1.0` (fully opaque)
    /// — existing callers that never call this see no behavior
    /// change.
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }
```

- [ ] **Step 4: Make `render` use `self.alpha`**

In `src/glitch.rs`'s `render`, replace:

```rust
                    buf.set(
                        x,
                        y,
                        Cell {
                            symbol: glyph,
                            fg: color,
                            bg: Color::Reset,
                            alpha: 1.0,
                            ..Default::default()
                        },
                    );
```

with:

```rust
                    buf.set(
                        x,
                        y,
                        Cell {
                            symbol: glyph,
                            fg: color,
                            bg: Color::Reset,
                            alpha: self.alpha,
                            ..Default::default()
                        },
                    );
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib glitch::`
Expected: all tests pass, including the 2 new ones and all 5
pre-existing `glitch::` tests (unaffected — none of them called
`.with_alpha()`, so they exercise the unchanged default-`1.0` path).

- [ ] **Step 6: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 7: Full workspace test**

Run: `cargo test --workspace`
Expected: full suite green, no regressions elsewhere.

- [ ] **Step 8: Mandatory visual review**

Run `tools/visual-snapshot` against `falcon` (its percussive-
maintenance mechanic triggers `GlitchBuffer`), scripting whatever key
sequence triggers the glitch. Capture and `Read` the resulting frame,
confirming the glitch still renders identically to before this change
(this task doesn't change any existing example's actual alpha value —
`with_alpha` is new and unused by any example yet — so the visual
check here is a pure regression confirmation, not a check of new
visual behavior).

- [ ] **Step 9: Commit**

```bash
git add src/glitch.rs
git commit -m "feat(core): add GlitchBuffer::with_alpha builder for partial transparency

Purely additive — GlitchBuffer::new() and render()'s signature are
unchanged, existing callers render identically (alpha defaults to
1.0, same as the prior hardcoded value).

Closes #115"
```

---

### Task 3: `Camera::project_line`'s `ProjectLineParams`

**Files:**
- Modify: `src/perspective.rs`
- Modify: `examples/falcon/falcon.rs`
- Modify: `examples/falcon/hud.rs`

**Interfaces:**
- Produces: `ProjectLineParams` struct, `Camera::project_line(&self,
  line: Line3, params: ProjectLineParams) -> Option<(u16, u16, u16,
  u16)>` (replaces the 8-positional-argument signature) — not consumed
  by any other task in this plan.

- [ ] **Step 1: Update the failing tests first**

In `src/perspective.rs`'s test module, replace all 4 `project_line`
calls. First:

```rust
        assert_eq!(
            cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0),
            None
        );
```

(in `project_line_returns_none_when_either_endpoint_is_behind_the_near_plane`)
becomes:

```rust
        assert_eq!(
            cam.project_line(
                line,
                ProjectLineParams {
                    center_x: 5.0,
                    center_y: 5.0,
                    screen_w: 10.0,
                    screen_h: 10.0,
                    subpixels_x: 2.0,
                    subpixels_y: 4.0,
                    min_scale: 0.0,
                }
            ),
            None
        );
```

Second, in
`project_line_returns_none_when_every_vertexs_scale_is_below_min_scale`:

```rust
        assert_eq!(
            cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.1),
            None
        );
```

becomes:

```rust
        assert_eq!(
            cam.project_line(
                line,
                ProjectLineParams {
                    center_x: 5.0,
                    center_y: 5.0,
                    screen_w: 10.0,
                    screen_h: 10.0,
                    subpixels_x: 2.0,
                    subpixels_y: 4.0,
                    min_scale: 0.1,
                }
            ),
            None
        );
```

Third, in
`project_line_projects_a_fully_visible_line_to_subpixel_coordinates`:

```rust
        let result = cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0);
```

becomes:

```rust
        let result = cam.project_line(
            line,
            ProjectLineParams {
                center_x: 5.0,
                center_y: 5.0,
                screen_w: 10.0,
                screen_h: 10.0,
                subpixels_x: 2.0,
                subpixels_y: 4.0,
                min_scale: 0.0,
            },
        );
```

Fourth, in
`project_line_clips_instead_of_saturating_an_off_screen_endpoint`:

```rust
        let result = cam
            .project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0)
            .expect("line crosses into the visible screen");
```

becomes:

```rust
        let result = cam
            .project_line(
                line,
                ProjectLineParams {
                    center_x: 5.0,
                    center_y: 5.0,
                    screen_w: 10.0,
                    screen_h: 10.0,
                    subpixels_x: 2.0,
                    subpixels_y: 4.0,
                    min_scale: 0.0,
                },
            )
            .expect("line crosses into the visible screen");
```

None of the 4 tests' assertions/expected values change — same inputs,
same expected outputs, only the call shape changes.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib perspective::`
Expected: FAIL to compile — `ProjectLineParams` doesn't exist yet, and
`project_line`'s signature doesn't accept it.

- [ ] **Step 3: Add `ProjectLineParams` and change `project_line`'s signature**

In `src/perspective.rs`, add this struct after `Polygon3`'s
definition (before the `Camera` struct):

```rust
/// Screen/subpixel parameters for `Camera::project_line` — bundles
/// everything except the line itself, since Falcon's HUD (the first
/// real consumer beyond the depth/perspective spike) call sites were
/// becoming hard to read with 7 trailing positional `f32`s.
#[derive(Clone, Copy, Debug)]
pub struct ProjectLineParams {
    /// Screen-space projection center, x.
    pub center_x: f32,
    /// Screen-space projection center, y.
    pub center_y: f32,
    /// Visible screen width in cells, for clipping.
    pub screen_w: f32,
    /// Visible screen height in cells, for clipping.
    pub screen_h: f32,
    /// Canvas subpixel columns per cell.
    pub subpixels_x: f32,
    /// Canvas subpixel rows per cell.
    pub subpixels_y: f32,
    /// Minimum projected scale for the line to be drawn at all.
    pub min_scale: f32,
}
```

Replace:

```rust
    #[allow(clippy::too_many_arguments)]
    pub fn project_line(
        &self,
        line: Line3,
        center_x: f32,
        center_y: f32,
        screen_w: f32,
        screen_h: f32,
        subpixels_x: f32,
        subpixels_y: f32,
        min_scale: f32,
    ) -> Option<(u16, u16, u16, u16)> {
        let (sx0, sy0, scale0) = self.project(line.start, center_x, center_y)?;
        let (sx1, sy1, scale1) = self.project(line.end, center_x, center_y)?;
        if scale0.max(scale1) < min_scale {
            return None;
        }
        let (cx0, cy0, cx1, cy1) = clip_to_screen(sx0, sy0, sx1, sy1, screen_w, screen_h)?;
        Some((
            (cx0 * subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
            (cy0 * subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
            (cx1 * subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
            (cy1 * subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
        ))
    }
```

with:

```rust
    pub fn project_line(
        &self,
        line: Line3,
        params: ProjectLineParams,
    ) -> Option<(u16, u16, u16, u16)> {
        let (sx0, sy0, scale0) = self.project(line.start, params.center_x, params.center_y)?;
        let (sx1, sy1, scale1) = self.project(line.end, params.center_x, params.center_y)?;
        if scale0.max(scale1) < params.min_scale {
            return None;
        }
        let (cx0, cy0, cx1, cy1) =
            clip_to_screen(sx0, sy0, sx1, sy1, params.screen_w, params.screen_h)?;
        Some((
            (cx0 * params.subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
            (cy0 * params.subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
            (cx1 * params.subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
            (cy1 * params.subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
        ))
    }
```

(the `#[allow(clippy::too_many_arguments)]` attribute is dropped — no
longer needed at 2 arguments.)

- [ ] **Step 4: Update Falcon's 2 call sites**

`examples/falcon/hud.rs` is a submodule of `falcon.rs`, included via
`use super::*;` at its top — it inherits `falcon.rs`'s top-level `use`
declarations and needs no import of its own. Add `ProjectLineParams`
to `falcon.rs`'s existing perspective import: replace

```rust
use ttui::perspective::{Camera, Line3, Point3};
```

with

```rust
use ttui::perspective::{Camera, Line3, Point3, ProjectLineParams};
```

In `examples/falcon/hud.rs`, find the `project_line` call (around line
31):

```rust
        if let Some((x0, y0, x1, y1)) = self.camera.project_line(
            seg,
            area.width as f32 / 2.0,
            area.height as f32 / 2.0,
            area.width as f32 - 1.0 / 2.0,
            area.height as f32 - 1.0 / 4.0,
            2.0,
            4.0,
            0.0,
        ) {
```

Replace with:

```rust
        if let Some((x0, y0, x1, y1)) = self.camera.project_line(
            seg,
            ProjectLineParams {
                center_x: area.width as f32 / 2.0,
                center_y: area.height as f32 / 2.0,
                screen_w: area.width as f32 - 1.0 / 2.0,
                screen_h: area.height as f32 - 1.0 / 4.0,
                subpixels_x: 2.0,
                subpixels_y: 4.0,
                min_scale: 0.0,
            },
        ) {
```

(everything after the closing `) {` on that line is unchanged — only
the call's argument list changes.)

In `examples/falcon/falcon.rs`, find its `project_line` call (the one
with the "cosmetic quirk" comment above it):

```rust
            if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                line,
                center_x,
                center_y,
                area.width as f32 - 1.0 / 2.0,
                area.height as f32 - 1.0 / 4.0,
                2.0,
                4.0,
                0.0,
            ) {
```

Replace with:

```rust
            if let Some((x0, y0, x1, y1)) = self.camera.project_line(
                line,
                ProjectLineParams {
                    center_x,
                    center_y,
                    screen_w: area.width as f32 - 1.0 / 2.0,
                    screen_h: area.height as f32 - 1.0 / 4.0,
                    subpixels_x: 2.0,
                    subpixels_y: 4.0,
                    min_scale: 0.0,
                },
            ) {
```

(`center_x`/`center_y` use Rust's field-init shorthand since local
variables of those exact names already exist at this call site —
confirmed from the surrounding code; everything after the closing `) {`
is unchanged.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib perspective::`
Expected: all `perspective::` tests pass, including the 4 updated
`project_line` tests.

- [ ] **Step 6: Build everything (catches Falcon's call sites)**

Run: `cargo build --all-targets`
Expected: succeeds — confirms both Falcon call sites compile against
the new signature.

- [ ] **Step 7: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 8: Full workspace test**

Run: `cargo test --workspace`
Expected: full suite green, no regressions elsewhere.

- [ ] **Step 9: Mandatory visual review**

`src/perspective.rs` isn't itself in `development-conventions.md`'s
mandated file list, but its only real consumer (Falcon's HUD) is.
Run `tools/visual-snapshot` against `falcon`, scripting whatever
interaction focuses each of the 3 HUD states (Hyperdrive/Sensors/
Weapons), and `Read` each captured frame — confirm every HUD line
renders identically to before this change (a pure signature
restructuring, no projection-math change, so every line should be
pixel-identical to a pre-change capture).

- [ ] **Step 10: Commit**

```bash
git add src/perspective.rs examples/falcon/falcon.rs examples/falcon/hud.rs
git commit -m "feat(core): replace Camera::project_line's 8 positional args with ProjectLineParams

Falcon's HUD is a real consumer now (2 call sites) — exactly the
condition the perspective-projection graduation deferred this change
on. Free now (pre-1.0); a public fn signature change is breaking once
ttui hits 1.0.

Closes #119"
```

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo test --workspace` — full suite green, including every
      updated test in `theme.rs`, `block.rs`, `glitch.rs`,
      `perspective.rs`.
- [ ] All 3 mandatory visual reviews (Tasks 1, 2, 3) were performed and
      their captures noted in the PR's Verification section.
- [ ] Issues #108, #115, #119 are closed (via each task's commit
      message `Closes #N`).
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this
      Arc's worktree branch to `main`, wait for all four required
      checks green, squash-merge, then remove the worktree via
      `ExitWorktree` (per the documented squash-merge resolution:
      verify via `gh pr view --json state,mergedAt,mergeCommit`, then
      retry with `discard_changes: true` if the tool's own ancestry
      check false-positives).
