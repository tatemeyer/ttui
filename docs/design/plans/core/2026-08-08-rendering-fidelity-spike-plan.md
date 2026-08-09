# Rendering Fidelity Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prototype six rendering-fidelity levers (truecolor depth, sub-cell canvas rendering, a full `CellStyle` attribute set, gradient color ramps, real alpha blending, and particle-trail polish) combined in one showcase example, to learn what TTUI's maximum achievable visual fidelity actually looks like and recommend which levers graduate into real, committed core Arcs.

**Architecture:** All new code lands as prototype-quality core modules (`src/canvas.rs`, `src/blend.rs`) plus additive extensions to the already-committed `CellStyle`/`render_diff` (from Arc 0 and the render-diff-performance arc), exercised together by one new example, `examples/render_spike.rs`. No existing public function signature changes. `CellStyle` gains four new fields — additive, not breaking, since every existing construction site already uses (or is updated in Task 5 to use) `..Default::default()`.

**Tech Stack:** Rust, `crossterm` 0.27 (raw-mode terminal I/O, `Color::Rgb` truecolor), existing `ttui` core (`buffer`, `easing`, `particles`, `layout`, `app`).

## Global Constraints

- **Research tag — TDD does NOT apply to any task in this plan**, per `.claude/rules/development-conventions.md`'s `research`-tagged exception. No task below has a failing-test-first step; verification is "it builds, it runs, you look at it," per the spec's own Testing section.
- `cargo fmt` / `cargo clippy --all-targets` is **not a hard gate** for these prototype files (spec's Verification section) — run it and fix free/trivial warnings, but do not block a task on it.
- **No existing public function signature changes.** `CellStyle`'s new fields are additive; every exhaustive `CellStyle { ... }` literal in the repo must be updated in the same task that adds the fields (Rust's exhaustiveness rule makes this mandatory, not optional).
- Windows-first, `crossterm`-only posture unchanged — no new dependencies.
- One worktree for this whole Arc, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- This Arc is `research`+`coding` tagged → **Gated** autonomy tier (`.claude/rules/git-github-standards.md`): ships as a PR to `main` with the four required checks green, squash-merged at the end — not a direct push, despite the TDD exemption above (TDD-exemption and merge-gating are separate axes).
- Spec being implemented: `docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md`.

---

### Task 1: Scaffold `examples/render_spike.rs` + color-depth audit (lever 1)

**Files:**
- Create: `examples/render_spike.rs`

**Interfaces:**
- Consumes: `ttui::app::{run, App}`, `ttui::buffer::{Cell, LayerStack}`, `ttui::layout::Rect` (all existing, unchanged).
- Produces: `RenderSpike` struct and `hue_to_rgb(hue: f32) -> Color` helper — later tasks extend this same file and reuse `hue_to_rgb`.

- [ ] **Step 1: Write the example**

```rust
// examples/render_spike.rs
//
// SPIKE PROTOTYPE for the rendering-fidelity spike
// (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md).
// Not a themed vision-doc app — a bare showcase proving out six
// rendering-fidelity levers together. This file grows across that
// spec's implementation plan; expect prototype-quality code throughout.

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::Rect;

struct RenderSpike {
    hue_shift: f32,
    quit: bool,
}

impl RenderSpike {
    fn new() -> Self {
        RenderSpike {
            hue_shift: 0.0,
            quit: false,
        }
    }
}

/// Cheap HSV(hue, 1.0, 1.0)->RGB — used only to paint smooth test
/// gradients in this spike, not a general color-space utility.
fn hue_to_rgb(hue: f32) -> Color {
    let h = hue.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h.rem_euclid(2.0) - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Color::Rgb {
        r: (r * 255.0) as u8,
        g: (g * 255.0) as u8,
        b: (b * 255.0) as u8,
    }
}

impl App for RenderSpike {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        // Lever 1: color-depth audit. A smooth 360-degree hue sweep
        // across the full width, animated by hue_shift. If this bands
        // into discrete steps instead of a smooth ramp, truecolor
        // isn't actually reaching the terminal — record that in the
        // spec's recommendations section (Task 9).
        for x in 0..area.width {
            let hue = (x as f32 / area.width.max(1) as f32) * 360.0 + self.hue_shift;
            let color = hue_to_rgb(hue);
            for y in 0..area.height {
                buf.set(
                    x,
                    y,
                    Cell {
                        symbol: '█',
                        fg: color,
                        bg: color,
                        ..Default::default()
                    },
                );
            }
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(50))
    }

    fn on_tick(&mut self, _elapsed: Duration) {
        self.hue_shift = (self.hue_shift + 2.0) % 360.0;
    }
}

fn main() -> std::io::Result<()> {
    let mut app = RenderSpike::new();
    run(&mut app)
}
```

- [ ] **Step 2: Build**

Run: `cargo build --example render_spike`
Expected: compiles cleanly.

- [ ] **Step 3: Run and audit color depth**

Run: `cargo run --example render_spike`
Expected: the whole screen fills with a smoothly animating rainbow
sweep. Look closely at the gradient: a smooth, continuous ramp with no
visible banded steps confirms 24-bit truecolor is reaching the
terminal. Visible discrete color bands would mean it's downsampling —
note either outcome now, it goes into Task 9's recommendations. Press
`q` to quit.

- [ ] **Step 4: Commit**

```bash
git add examples/render_spike.rs
git commit -m "research(core): scaffold render_spike example, audit color depth"
```

---

### Task 2: `Canvas` primitive — half-block mode (lever 2, part 1)

**Files:**
- Create: `src/canvas.rs`
- Modify: `src/lib.rs` (add `pub mod canvas;`)

**Interfaces:**
- Consumes: `ttui::buffer::{Buffer, Cell, CellStyle}` (existing).
- Produces: `CanvasMode` enum, `Canvas` struct with `new`, `set_pixel`,
  `clear_pixel`, `blit` (half-block mode only this task) — Task 3 adds
  `line`/`rect`/`fill_rect` and braille-mode `blit`.

- [ ] **Step 1: Write the module**

```rust
//! Sub-cell rendering primitive (half-block + braille) — SPIKE
//! PROTOTYPE for the rendering-fidelity spike
//! (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md).
//! Not a committed, stable API: expect this to be rewritten once the
//! spike's recommendations are acted on.

use crate::buffer::{Buffer, Cell, CellStyle};
use crossterm::style::Color;

/// Which sub-cell rasterization technique a `Canvas` uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasMode {
    /// 1x2 subpixels per cell, full 2-color fidelity (▀/▄/█).
    HalfBlock,
    /// 2x4 subpixels per cell, one fg color per cell (braille glyphs).
    Braille,
}

/// A higher-resolution drawing surface that rasterizes into ordinary
/// `Cell`s via `blit`. See module docs — spike prototype, not a
/// committed API.
pub struct Canvas {
    width: u16,  // in cells
    height: u16, // in cells
    mode: CanvasMode,
    subpixels_x: u16,
    subpixels_y: u16,
    grid: Vec<Option<Color>>, // len = grid_width() * grid_height()
}

impl Canvas {
    /// Creates a blank `width`x`height`-cell canvas in `mode`.
    pub fn new(width: u16, height: u16, mode: CanvasMode) -> Self {
        let (subpixels_x, subpixels_y) = match mode {
            CanvasMode::HalfBlock => (1, 2),
            CanvasMode::Braille => (2, 4),
        };
        let grid_w = width as usize * subpixels_x as usize;
        let grid_h = height as usize * subpixels_y as usize;
        Canvas {
            width,
            height,
            mode,
            subpixels_x,
            subpixels_y,
            grid: vec![None; grid_w * grid_h],
        }
    }

    fn grid_width(&self) -> u16 {
        self.width * self.subpixels_x
    }

    fn grid_height(&self) -> u16 {
        self.height * self.subpixels_y
    }

    fn index(&self, x: u16, y: u16) -> usize {
        y as usize * self.grid_width() as usize + x as usize
    }

    /// Sets the subpixel at `(x, y)` (subpixel coordinates) to `color`.
    /// Out-of-bounds coordinates are silently ignored.
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Color) {
        if x < self.grid_width() && y < self.grid_height() {
            let idx = self.index(x, y);
            self.grid[idx] = Some(color);
        }
    }

    /// Clears the subpixel at `(x, y)` back to transparent.
    pub fn clear_pixel(&mut self, x: u16, y: u16) {
        if x < self.grid_width() && y < self.grid_height() {
            let idx = self.index(x, y);
            self.grid[idx] = None;
        }
    }

    /// Rasterizes this canvas into `buf` at cell offset `(x, y)`.
    /// Cells with no set subpixels are left untouched (transparent).
    pub fn blit(&self, buf: &mut Buffer, x: u16, y: u16) {
        match self.mode {
            CanvasMode::HalfBlock => self.blit_half_block(buf, x, y),
            CanvasMode::Braille => { /* added in Task 3 */ }
        }
    }

    fn blit_half_block(&self, buf: &mut Buffer, ox: u16, oy: u16) {
        for cy in 0..self.height {
            for cx in 0..self.width {
                let top = self.grid[self.index(cx, cy * 2)];
                let bottom = self.grid[self.index(cx, cy * 2 + 1)];
                let cell = match (top, bottom) {
                    (None, None) => continue, // transparent: leave buf untouched
                    (Some(t), None) => Cell {
                        symbol: '▀',
                        fg: t,
                        bg: Color::Reset,
                        style: CellStyle::default(),
                    },
                    (None, Some(b)) => Cell {
                        symbol: '▄',
                        fg: b,
                        bg: Color::Reset,
                        style: CellStyle::default(),
                    },
                    (Some(t), Some(b)) if t == b => Cell {
                        symbol: '█',
                        fg: t,
                        bg: t,
                        style: CellStyle::default(),
                    },
                    (Some(t), Some(b)) => Cell {
                        symbol: '▀',
                        fg: t,
                        bg: b,
                        style: CellStyle::default(),
                    },
                };
                let bx = ox + cx;
                let by = oy + cy;
                if bx < buf.width && by < buf.height {
                    buf.set(bx, by, cell);
                }
            }
        }
    }
}
```

- [ ] **Step 2: Register the module**

In `src/lib.rs`, add near the other `pub mod` lines (alphabetical, next
to `pub mod camera;`/`pub mod buffer;`):

```rust
/// Sub-cell rendering primitive (half-block + braille) — spike
/// prototype, not a committed API.
pub mod canvas;
```

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: compiles (the `CanvasMode::Braille => {}` arm is an
intentional no-op placeholder completed in Task 3 — this compiles fine
as an empty match arm, it is not a "TBD" left in shipped behavior).

- [ ] **Step 4: Commit**

```bash
git add src/canvas.rs src/lib.rs
git commit -m "research(core): add Canvas half-block sub-cell rendering prototype"
```

---

### Task 3: `Canvas` — braille mode + shape helpers (lever 2, part 2)

**Files:**
- Modify: `src/canvas.rs`

**Interfaces:**
- Consumes: `Canvas` from Task 2.
- Produces: `Canvas::line`, `Canvas::rect`, `Canvas::fill_rect`, and a
  working `CanvasMode::Braille` branch in `blit` — Task 4 uses all of
  these.

- [ ] **Step 1: Replace the `Braille` no-op arm in `blit`**

In `src/canvas.rs`, change:

```rust
    pub fn blit(&self, buf: &mut Buffer, x: u16, y: u16) {
        match self.mode {
            CanvasMode::HalfBlock => self.blit_half_block(buf, x, y),
            CanvasMode::Braille => { /* added in Task 3 */ }
        }
    }
```

to:

```rust
    pub fn blit(&self, buf: &mut Buffer, x: u16, y: u16) {
        match self.mode {
            CanvasMode::HalfBlock => self.blit_half_block(buf, x, y),
            CanvasMode::Braille => self.blit_braille(buf, x, y),
        }
    }
```

- [ ] **Step 2: Add `blit_braille` and the shape helpers**

Add to `src/canvas.rs`, after `blit_half_block`:

```rust
    fn blit_braille(&self, buf: &mut Buffer, ox: u16, oy: u16) {
        // Braille dot bit layout (Unicode "Braille Patterns" block,
        // U+2800): bit0/bit3 = row0 col0/col1, bit1/bit4 = row1,
        // bit2/bit5 = row2, bit6/bit7 = row3.
        const DOT_BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
        for cy in 0..self.height {
            for cx in 0..self.width {
                let mut mask: u8 = 0;
                let mut color: Option<Color> = None;
                for row in 0..4u16 {
                    for col in 0..2u16 {
                        let px = cx * 2 + col;
                        let py = cy * 4 + row;
                        if let Some(c) = self.grid[self.index(px, py)] {
                            mask |= DOT_BITS[row as usize][col as usize];
                            color = Some(c); // last-write-wins per cell
                        }
                    }
                }
                if mask == 0 {
                    continue; // transparent
                }
                let symbol = char::from_u32(0x2800 + mask as u32).unwrap();
                let bx = ox + cx;
                let by = oy + cy;
                if bx < buf.width && by < buf.height {
                    buf.set(
                        bx,
                        by,
                        Cell {
                            symbol,
                            fg: color.unwrap(),
                            bg: Color::Reset,
                            style: CellStyle::default(),
                        },
                    );
                }
            }
        }
    }

    /// Draws a straight line between two subpixel points (Bresenham).
    pub fn line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: Color) {
        let (mut x0, mut y0) = (x0 as i32, y0 as i32);
        let (x1, y1) = (x1 as i32, y1 as i32);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if x0 >= 0 && y0 >= 0 {
                self.set_pixel(x0 as u16, y0 as u16, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draws a rectangle outline with top-left at `(x, y)` (subpixel
    /// coordinates).
    pub fn rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        if w == 0 || h == 0 {
            return;
        }
        self.line(x, y, x + w - 1, y, color);
        self.line(x, y + h - 1, x + w - 1, y + h - 1, color);
        self.line(x, y, x, y + h - 1, color);
        self.line(x + w - 1, y, x + w - 1, y + h - 1, color);
    }

    /// Fills a solid rectangle with top-left at `(x, y)` (subpixel
    /// coordinates).
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        for row in y..y + h {
            for col in x..x + w {
                self.set_pixel(col, row, color);
            }
        }
    }
```

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add src/canvas.rs
git commit -m "research(core): add Canvas braille mode and line/rect/fill_rect"
```

---

### Task 4: Wire `Canvas` into `render_spike.rs` — half-block gauge + braille plot

**Files:**
- Modify: `examples/render_spike.rs`

**Interfaces:**
- Consumes: `ttui::canvas::{Canvas, CanvasMode}` (Tasks 2-3),
  `ttui::easing::lerp_color` (existing), `ttui::layout::{Constraint,
  Direction, Layout}` (existing), `hue_to_rgb` (Task 1, unchanged).
- Produces: `RenderSpike::render_gauge`/`render_plot` methods and
  `gauge_phase`/`plot_phase` fields — Task 8 reuses the row layout
  established here.

- [ ] **Step 1: Add imports and state fields**

In `examples/render_spike.rs`, change the `use` block to:

```rust
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::canvas::{Canvas, CanvasMode};
use ttui::easing::lerp_color;
use ttui::layout::{Constraint, Direction, Layout, Rect};
```

and change the `RenderSpike` struct and constructor to:

```rust
struct RenderSpike {
    hue_shift: f32,
    gauge_phase: f32,
    plot_phase: f32,
    quit: bool,
}

impl RenderSpike {
    fn new() -> Self {
        RenderSpike {
            hue_shift: 0.0,
            gauge_phase: 0.0,
            plot_phase: 0.0,
            quit: false,
        }
    }
```

- [ ] **Step 2: Add the gauge and plot renderers**

Add these methods inside `impl RenderSpike` (after `new`):

```rust
    fn render_gauge(&self, area: Rect, buf: &mut LayerStack) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::HalfBlock);
        let subpixel_height = area.height * 2;
        let fill = self.gauge_phase.sin() * 0.5 + 0.5; // 0..1
        let filled = (subpixel_height as f32 * fill).round() as u16;
        for row in 0..filled {
            let t = row as f32 / subpixel_height.max(1) as f32;
            let color = lerp_color(
                Color::Rgb { r: 220, g: 40, b: 40 },
                Color::Rgb { r: 40, g: 220, b: 90 },
                t,
            );
            for col in 0..area.width {
                canvas.set_pixel(col, subpixel_height - 1 - row, color);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }

    fn render_plot(&self, area: Rect, buf: &mut LayerStack) {
        if area.width < 2 || area.height == 0 {
            return;
        }
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let grid_w = area.width * 2;
        let grid_h = area.height * 4;
        let sample = |gx: u16| -> f32 { (gx as f32 * 0.25 + self.plot_phase).sin() };
        for gx in 0..grid_w.saturating_sub(1) {
            let y0 = grid_h - 1 - ((sample(gx) * 0.5 + 0.5) * (grid_h - 1) as f32).round() as u16;
            let y1 =
                grid_h - 1 - ((sample(gx + 1) * 0.5 + 0.5) * (grid_h - 1) as f32).round() as u16;
            canvas.line(gx, y0, gx + 1, y1, Color::Rgb { r: 90, g: 180, b: 255 });
        }
        canvas.blit(buf, area.x, area.y);
    }
```

- [ ] **Step 3: Call them from `view` and update phases in `on_tick`**

Replace the `view` method body with:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        for x in 0..area.width {
            let hue = (x as f32 / area.width.max(1) as f32) * 360.0 + self.hue_shift;
            let color = hue_to_rgb(hue);
            for y in 0..area.height {
                buf.set(
                    x,
                    y,
                    Cell {
                        symbol: '█',
                        fg: color,
                        bg: color,
                        ..Default::default()
                    },
                );
            }
        }
    }
```

(this stays the Task-1 body for now — Task 8 replaces it wholesale with
the full assembled scene, so no further layout wiring is needed here
yet).

Change `on_tick` to:

```rust
    fn on_tick(&mut self, elapsed: Duration) {
        self.hue_shift = (self.hue_shift + 2.0) % 360.0;
        self.gauge_phase += elapsed.as_secs_f32() * 1.5;
        self.plot_phase += elapsed.as_secs_f32() * 4.0;
    }
```

- [ ] **Step 4: Temporarily call the new renderers to verify them visually**

This step is throwaway verification, replaced by Task 8's real layout.
In `view`, after the existing hue-sweep loop, temporarily add:

```rust
        self.render_gauge(
            Rect { x: 2, y: 2, width: 10, height: 8 },
            buf,
        );
        self.render_plot(
            Rect { x: 14, y: 2, width: 30, height: 8 },
            buf,
        );
```

- [ ] **Step 5: Build and run**

Run: `cargo build --example render_spike && cargo run --example render_spike`
Expected: a small filled gauge (bottom red, top green, animating up
and down) appears near the top-left, and a smoothly-curving braille
sine wave appears to its right, both animating. Confirms both `Canvas`
modes work end-to-end. Press `q` to quit.

- [ ] **Step 6: Commit**

```bash
git add examples/render_spike.rs
git commit -m "research(core): wire half-block gauge and braille plot into render_spike"
```

---

### Task 5: Full `CellStyle` attribute set (lever 3)

**Files:**
- Modify: `src/buffer.rs` (`CellStyle` fields)
- Modify: `src/terminal.rs` (`render_diff` wiring, test helper)
- Modify: `src/widgets/block.rs:63`, `benches/render.rs:48`,
  `examples/smash_crabs/target_smash.rs:89`,
  `examples/smash_crabs/smash_crabs.rs:336`,
  `examples/launcher/main.rs:161` (exhaustive `CellStyle{}` literal
  fixups — required by Rust's exhaustiveness rule once new fields
  exist, not optional)
- Modify: `examples/render_spike.rs` (attribute showcase row)

**Interfaces:**
- Consumes: existing `CellStyle` (Arc 0), existing `render_diff`
  (render-diff-performance arc).
- Produces: `CellStyle.underline`/`.italic`/`.reverse`/`.strikethrough`
  fields — Task 8 uses these in the gradient-ring border.

**Note on `dim`:** the spec lists five candidate attributes including
`dim`, but terminal "intensity" is really one tri-state axis
(normal/bold/dim), not independently-togglable bools — `bold` already
exists as its own field from Arc 0. Adding a separate `dim: bool`
would let `bold: true, dim: true` be constructed even though that's not
a real terminal state. This task deliberately does **not** add `dim`
and instead records the finding for Task 9's recommendations: a real
follow-up Arc should model intensity as an enum
(`Intensity::{Normal,Bold,Dim}`), not `bold: bool` plus a bolted-on
`dim: bool`.

- [ ] **Step 1: Extend `CellStyle`**

In `src/buffer.rs`, change:

```rust
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CellStyle {
    /// Whether the cell renders bold.
    pub bold: bool,
}
```

to:

```rust
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

- [ ] **Step 2: Fix the six exhaustive `CellStyle { .. }` literal sites**

Each of these currently constructs `CellStyle` without the four new
fields, which no longer compiles once Step 1 lands. Append
`..Default::default()` to each:

`src/widgets/block.rs:63` — change
```rust
        let plain = || Cell {
            symbol: ' ',
            fg,
            bg,
            style: CellStyle { bold: border_bold },
        };
```
to
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

`src/terminal.rs` (the `render_diff_tests::d()` helper, ~line 133) —
change `style: CellStyle { bold },` to
`style: CellStyle { bold, ..Default::default() },`.

`benches/render.rs:48` — change `style: CellStyle { bold: false },` to
`style: CellStyle { bold: false, ..Default::default() },`.

`examples/smash_crabs/target_smash.rs:89` — change
`style: CellStyle { bold: true },` to
`style: CellStyle { bold: true, ..Default::default() },`.

`examples/smash_crabs/smash_crabs.rs:336` — same fixup as above.

`examples/launcher/main.rs:161` — change `style: CellStyle { bold },`
to `style: CellStyle { bold, ..Default::default() },`.

- [ ] **Step 3: Wire the new attributes into `render_diff`**

In `src/terminal.rs`, replace the `render_diff` function body with:

```rust
pub fn render_diff(writer: &mut impl Write, diffs: &[CellDiff]) -> std::io::Result<()> {
    let mut last_pos: Option<(u16, u16)> = None;
    let mut last_fg: Option<Color> = None;
    let mut last_bg: Option<Color> = None;
    let mut last_bold: Option<bool> = None;
    let mut last_underline: Option<bool> = None;
    let mut last_italic: Option<bool> = None;
    let mut last_reverse: Option<bool> = None;
    let mut last_strikethrough: Option<bool> = None;

    for d in diffs {
        let contiguous =
            matches!(last_pos, Some((px, py)) if py == d.y && d.x.checked_sub(1) == Some(px));
        if !contiguous {
            queue!(writer, cursor::MoveTo(d.x, d.y))?;
        }

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
        let underline = d.cell.style.underline;
        if last_underline != Some(underline) {
            let attr = if underline {
                Attribute::Underlined
            } else {
                Attribute::NoUnderline
            };
            queue!(writer, SetAttribute(attr))?;
            last_underline = Some(underline);
        }
        let italic = d.cell.style.italic;
        if last_italic != Some(italic) {
            let attr = if italic {
                Attribute::Italic
            } else {
                Attribute::NoItalic
            };
            queue!(writer, SetAttribute(attr))?;
            last_italic = Some(italic);
        }
        let reverse = d.cell.style.reverse;
        if last_reverse != Some(reverse) {
            let attr = if reverse {
                Attribute::Reverse
            } else {
                Attribute::NoReverse
            };
            queue!(writer, SetAttribute(attr))?;
            last_reverse = Some(reverse);
        }
        let strikethrough = d.cell.style.strikethrough;
        if last_strikethrough != Some(strikethrough) {
            let attr = if strikethrough {
                Attribute::CrossedOut
            } else {
                Attribute::NotCrossedOut
            };
            queue!(writer, SetAttribute(attr))?;
            last_strikethrough = Some(strikethrough);
        }
        if last_fg != Some(d.cell.fg) {
            queue!(writer, SetForegroundColor(d.cell.fg))?;
            last_fg = Some(d.cell.fg);
        }
        if last_bg != Some(d.cell.bg) {
            queue!(writer, SetBackgroundColor(d.cell.bg))?;
            last_bg = Some(d.cell.bg);
        }
        queue!(writer, Print(d.cell.symbol))?;
        last_pos = Some((d.x, d.y));
    }
    Ok(())
}
```

- [ ] **Step 4: Build**

Run: `cargo build --all-targets`
Expected: compiles cleanly (this touches `benches/render.rs`, so
`--all-targets` matters here, not just `cargo build`).

- [ ] **Step 5: Add an attribute showcase row to `render_spike.rs`**

In `examples/render_spike.rs`, add this method to `impl RenderSpike`:

```rust
    fn render_attribute_showcase(&self, area: Rect, buf: &mut LayerStack) {
        use ttui::buffer::CellStyle;
        let words: [(&str, CellStyle); 4] = [
            (
                "UNDERLINE",
                CellStyle {
                    underline: true,
                    ..Default::default()
                },
            ),
            (
                "ITALIC",
                CellStyle {
                    italic: true,
                    ..Default::default()
                },
            ),
            (
                "REVERSE",
                CellStyle {
                    reverse: true,
                    ..Default::default()
                },
            ),
            (
                "STRIKETHROUGH",
                CellStyle {
                    strikethrough: true,
                    ..Default::default()
                },
            ),
        ];
        let mut x = area.x;
        for (word, style) in words {
            for ch in word.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf.set(
                    x,
                    area.y,
                    Cell {
                        symbol: ch,
                        fg: Color::Reset,
                        bg: Color::Reset,
                        style,
                    },
                );
                x += 1;
            }
            x += 2; // gap between words
        }
    }
```

This is wired into the real layout in Task 8; for now it exists as a
method other tasks can call.

- [ ] **Step 6: Build**

Run: `cargo build --example render_spike`
Expected: compiles cleanly (the method is unused until Task 8 calls
it — an `unused` warning here is expected and fine, not a gate per
this plan's Global Constraints).

- [ ] **Step 7: Commit**

```bash
git add src/buffer.rs src/terminal.rs src/widgets/block.rs benches/render.rs \
  examples/smash_crabs/target_smash.rs examples/smash_crabs/smash_crabs.rs \
  examples/launcher/main.rs examples/render_spike.rs
git commit -m "research(core): extend CellStyle with underline/italic/reverse/strikethrough"
```

---

### Task 6: Alpha-blending prototype (lever 5) — `src/blend.rs`

**Files:**
- Create: `src/blend.rs`
- Modify: `src/lib.rs` (add `pub mod blend;`)

**Interfaces:**
- Consumes: `ttui::buffer::{Buffer, Cell}`, `ttui::easing::lerp_color`
  (existing).
- Produces: `blend_over(base, overlay, alpha) -> Buffer`,
  `fade_toward(buf, target, factor) -> Buffer` — Task 7 uses both.

- [ ] **Step 1: Write the module**

```rust
//! Alpha-blending prototype for the rendering-fidelity spike
//! (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md).
//! Spike-only: not a committed replacement for `LayerStack::composite`'s
//! hard-cutout compositing rule.

use crate::buffer::{Buffer, Cell};
use crate::easing::lerp_color;
use crossterm::style::Color;

/// Blends `overlay`'s non-default cells over `base`, interpolating
/// fg/bg color by `alpha` (0 = base only, 1 = overlay only) via
/// `easing::lerp_color`. `overlay` cells equal to `Cell::default()`
/// are treated as "painted nothing" and skipped entirely — the same
/// transparency rule `LayerStack` already uses. The overlay's glyph
/// replaces the base's once `alpha >= 0.5` (glyphs don't blend; this
/// is a documented spike simplification). Iterates the smaller of the
/// two buffers' dimensions if they differ in size.
pub fn blend_over(base: &Buffer, overlay: &Buffer, alpha: f32) -> Buffer {
    let mut out = base.clone();
    for y in 0..base.height.min(overlay.height) {
        for x in 0..base.width.min(overlay.width) {
            let ov = overlay.get(x, y);
            if *ov == Cell::default() {
                continue;
            }
            let b = base.get(x, y);
            let blended = Cell {
                symbol: if alpha >= 0.5 { ov.symbol } else { b.symbol },
                fg: lerp_color(b.fg, ov.fg, alpha),
                bg: lerp_color(b.bg, ov.bg, alpha),
                style: if alpha >= 0.5 { ov.style } else { b.style },
            };
            out.set(x, y, blended);
        }
    }
    out
}

/// Interpolates every non-default cell's fg/bg toward `target` by
/// `factor` (0 = unchanged, 1 = fully `target`), collapsing a cell all
/// the way to `Cell::default()` once both channels are within 2 RGB
/// steps of `target` — lets a fully-faded trail cell become
/// transparent again.
///
/// **SPIKE FINDING:** this only works because `target` is `Rgb` —
/// `easing::lerp_color` falls back to its `to` argument immediately
/// for any non-`Rgb` color, so fading toward `Color::Reset` (true
/// transparency) is NOT gradual today. See this spec's
/// recommendations section (Task 9).
pub fn fade_toward(buf: &Buffer, target: Color, factor: f32) -> Buffer {
    let mut out = buf.clone();
    let close_enough = |a: Color| -> bool {
        matches!(
            (a, target),
            (
                Color::Rgb { r: r1, g: g1, b: b1 },
                Color::Rgb { r: r2, g: g2, b: b2 },
            ) if r1.abs_diff(r2) <= 2 && g1.abs_diff(g2) <= 2 && b1.abs_diff(b2) <= 2
        )
    };
    for y in 0..buf.height {
        for x in 0..buf.width {
            let c = buf.get(x, y);
            if *c == Cell::default() {
                continue;
            }
            let fg = lerp_color(c.fg, target, factor);
            let bg = lerp_color(c.bg, target, factor);
            if close_enough(fg) && close_enough(bg) {
                out.set(x, y, Cell::default());
            } else {
                out.set(
                    x,
                    y,
                    Cell {
                        fg,
                        bg,
                        ..c.clone()
                    },
                );
            }
        }
    }
    out
}
```

- [ ] **Step 2: Register the module**

In `src/lib.rs`, add:

```rust
/// Alpha-blending prototype — spike prototype, not a committed API.
pub mod blend;
```

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add src/blend.rs src/lib.rs
git commit -m "research(core): add blend_over/fade_toward alpha-blending prototype"
```

---

### Task 7: Particle-trail polish (lever 6) — wire into `render_spike.rs`

**Files:**
- Modify: `examples/render_spike.rs`

**Interfaces:**
- Consumes: `ttui::particles::{Particle, ParticleSystem}` (existing,
  unchanged), `ttui::blend::{blend_over, fade_toward}` (Task 6).
- Produces: `RenderSpike.particles`/`.trail` fields, particle-burst
  spawn on Space — Task 8's final scene assembly calls the blend step
  this task adds.

- [ ] **Step 1: Add imports and state**

In `examples/render_spike.rs`, add to the `use` block:

```rust
use std::time::Duration;
use ttui::blend::{blend_over, fade_toward};
use ttui::buffer::Buffer;
use ttui::particles::{Particle, ParticleSystem};
```

Add fields to `RenderSpike` and initialize them in `new`:

```rust
struct RenderSpike {
    hue_shift: f32,
    gauge_phase: f32,
    plot_phase: f32,
    particles: ParticleSystem,
    trail: Buffer,
    quit: bool,
}

impl RenderSpike {
    fn new() -> Self {
        RenderSpike {
            hue_shift: 0.0,
            gauge_phase: 0.0,
            plot_phase: 0.0,
            particles: ParticleSystem::new(),
            trail: Buffer::new(160, 50),
            quit: false,
        }
    }
```

- [ ] **Step 2: Spawn a burst on Space**

In `update`, change the `match k.code` block to:

```rust
        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char(' ') => {
                let center = (40.0, 15.0);
                for i in 0..16 {
                    let angle = i as f32 * (std::f32::consts::TAU / 16.0);
                    self.particles.spawn(Particle {
                        x: center.0,
                        y: center.1,
                        vx: angle.cos() * 20.0,
                        vy: angle.sin() * 10.0,
                        symbol: '*',
                        color: Color::Rgb {
                            r: 255,
                            g: 180,
                            b: 40,
                        },
                        lifetime: Duration::from_millis(700),
                        age: Duration::ZERO,
                    });
                }
            }
            _ => {}
        }
```

- [ ] **Step 3: Decay the trail and update particles in `on_tick`**

Change `on_tick` to:

```rust
    fn on_tick(&mut self, elapsed: Duration) {
        self.hue_shift = (self.hue_shift + 2.0) % 360.0;
        self.gauge_phase += elapsed.as_secs_f32() * 1.5;
        self.plot_phase += elapsed.as_secs_f32() * 4.0;
        self.particles.update(elapsed);
        self.trail = fade_toward(&self.trail, Color::Rgb { r: 0, g: 0, b: 0 }, 0.2);
        self.particles.render(&mut self.trail);
    }
```

- [ ] **Step 4: Blend the trail over the rendered scene**

Add this method to `impl RenderSpike`:

```rust
    fn blend_trail(&self, buf: &mut LayerStack) {
        let scene = buf.composite();
        let scene = blend_over(&scene, &self.trail, 1.0);
        *buf.layer_mut(0) = scene;
    }
```

This is called at the end of `view` starting in Task 8, once the full
scene (gauge, plot, ring, attribute row) has been painted into `buf`'s
base layer — trails need to blend over the *finished* frame, not an
empty one.

- [ ] **Step 5: Build**

Run: `cargo build --example render_spike`
Expected: compiles cleanly (`blend_trail` is unused until Task 8 calls
it — expected, not a gate).

- [ ] **Step 6: Commit**

```bash
git add examples/render_spike.rs
git commit -m "research(core): add particle-burst trails via blend_over/fade_toward"
```

---

### Task 8: Gradient-bordered ring + final scene assembly (lever 4 + integration)

**Files:**
- Modify: `examples/render_spike.rs`

**Interfaces:**
- Consumes: everything from Tasks 1-7 (`hue_to_rgb`, `render_gauge`,
  `render_plot`, `render_attribute_showcase`, `blend_trail`).
- Produces: the final assembled `view()` — Task 9 only adds
  instrumentation around this, no further structural changes.

- [ ] **Step 1: Add the gradient-ring border helper**

Add this free function to `examples/render_spike.rs` (near
`hue_to_rgb`):

```rust
fn draw_gradient_ring(area: Rect, buf: &mut LayerStack, hue_shift: f32) {
    use ttui::buffer::CellStyle;
    if area.width < 2 || area.height < 2 {
        return;
    }
    let ring_cell = |x: u16, y: u16, symbol: char| -> Cell {
        let t = (x as f32 - area.x as f32) / area.width.max(1) as f32
            + (y as f32 - area.y as f32) / area.height.max(1) as f32;
        Cell {
            symbol,
            fg: hue_to_rgb(t * 180.0 + hue_shift),
            bg: Color::Reset,
            style: CellStyle {
                bold: true,
                ..Default::default()
            },
        }
    };
    for x in area.x..area.x + area.width {
        buf.set(x, area.y, ring_cell(x, area.y, '▀'));
        buf.set(
            x,
            area.y + area.height - 1,
            ring_cell(x, area.y + area.height - 1, '▄'),
        );
    }
    for y in area.y..area.y + area.height {
        buf.set(area.x, y, ring_cell(area.x, y, '█'));
        buf.set(
            area.x + area.width - 1,
            y,
            ring_cell(area.x + area.width - 1, y, '█'),
        );
    }
}
```

- [ ] **Step 2: Replace `view` with the full assembled scene**

Replace the entire `view` method with:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        draw_gradient_ring(area, buf, self.hue_shift);
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fill(1)],
        )
        .split(inner);

        self.render_attribute_showcase(rows[0], buf);

        let cols = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Percentage(50), Constraint::Fill(1)],
        )
        .split(rows[1]);
        self.render_gauge(cols[0], buf);
        self.render_plot(cols[1], buf);

        self.blend_trail(buf);
    }
```

- [ ] **Step 3: Remove the now-redundant Task 1 hue-sweep fill**

The full-screen hue sweep from Task 1 (looping `buf.set` with `'█'`
across every cell) is superseded by `draw_gradient_ring` — confirm it
no longer appears anywhere in `view` (it was already fully replaced by
Step 2 above; this step is a read-back check, not an edit).

- [ ] **Step 4: Build and run the full showcase**

Run: `cargo build --example render_spike && cargo run --example render_spike`
Expected: a gradient-colored border frames the whole screen; inside it,
a row of differently-styled words (underline/italic/reverse/
strikethrough); below that, the half-block gauge on the left and the
braille sine plot on the right, all animating continuously. Press
Space repeatedly — particle bursts radiate from a fixed point and fade
out smoothly (color blending toward black, not a hard cutoff) rather
than vanishing abruptly. Press `q` to quit. This is the concrete
"looks visibly better than `main`" bar from the spec's success
criteria — compare against `cargo run --example demo` if useful.

- [ ] **Step 5: Commit**

```bash
git add examples/render_spike.rs
git commit -m "research(core): assemble full render_spike showcase scene"
```

---

### Task 9: Frame-time measurement + recommendations write-up

**Files:**
- Modify: `examples/render_spike.rs` (temporary instrumentation)
- Modify: `docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md`
  (append findings)

**Interfaces:**
- Consumes: the finished scene from Task 8.
- Produces: printed frame-time/diff-size numbers (used only to write
  the recommendations section, not kept as permanent code) and the
  spec's new "Recommendations" section.

- [ ] **Step 1: Add temporary frame-time instrumentation**

In `examples/render_spike.rs`, this plan's scene already renders via
`ttui::app::run`, which does not expose per-frame timing. Rather than
modifying `app::run` (out of scope — no existing signature changes),
measure by running a short, separate timing harness in `main`:

```rust
fn main() -> std::io::Result<()> {
    if std::env::args().any(|a| a == "--bench") {
        return bench_frame_cost();
    }
    let mut app = RenderSpike::new();
    run(&mut app)
}

/// Ad hoc timing harness for this spike's recommendations write-up —
/// not a criterion benchmark, not kept as a permanent measurement
/// tool. Builds the densest frame this scene produces (mid-burst, all
/// six levers active) and times `view` + `composite` + `render_diff`
/// directly, bypassing the terminal.
fn bench_frame_cost() -> std::io::Result<()> {
    use ttui::buffer::{diff, LayerStack};
    use ttui::terminal::render_diff;

    let mut app = RenderSpike::new();
    for i in 0..16 {
        let angle = i as f32 * (std::f32::consts::TAU / 16.0);
        app.particles.spawn(Particle {
            x: 40.0,
            y: 15.0,
            vx: angle.cos() * 20.0,
            vy: angle.sin() * 10.0,
            symbol: '*',
            color: Color::Rgb {
                r: 255,
                g: 180,
                b: 40,
            },
            lifetime: Duration::from_millis(700),
            age: Duration::ZERO,
        });
    }
    app.on_tick(Duration::from_millis(16));

    let area = Rect {
        x: 0,
        y: 0,
        width: 120,
        height: 40,
    };
    let mut prev = ttui::buffer::Buffer::new(area.width, area.height);
    let start = std::time::Instant::now();
    const FRAMES: u32 = 200;
    let mut total_diffs = 0usize;
    for _ in 0..FRAMES {
        let mut stack = LayerStack::new(area.width, area.height);
        app.view(area, &mut stack);
        let next = stack.composite();
        let diffs = diff(&prev, &next);
        total_diffs += diffs.len();
        let mut sink = Vec::new();
        render_diff(&mut sink, &diffs)?;
        prev = next;
        app.on_tick(Duration::from_millis(16));
    }
    let elapsed = start.elapsed();
    println!(
        "{FRAMES} frames in {:?} ({:?}/frame avg), avg {} diffed cells/frame",
        elapsed,
        elapsed / FRAMES,
        total_diffs / FRAMES as usize
    );
    Ok(())
}
```

- [ ] **Step 2: Run the timing harness**

Run: `cargo run --example render_spike -- --bench`
Expected: prints per-frame timing and average diffed-cell count for the
densest real frame this scene produces. Record the printed numbers —
they go directly into Step 4's recommendations section.

- [ ] **Step 3: Build check**

Run: `cargo build --examples`
Expected: all examples, including `render_spike`, still compile.

- [ ] **Step 4: Append the recommendations section to the spec**

Append to
`docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md`,
after its "Explicitly deferred / open questions for future revisions"
section:

```markdown
## Recommendations (post-spike)

Written after running `examples/render_spike.rs` and its `--bench`
timing harness.

- **Color depth (lever 1):** [fill in: smooth/banded — record what
  Task 1's visual check actually showed].
- **Frame cost:** [fill in the printed `--bench` numbers from Step 2]
  — evaluate against Rev A's tactile-responsiveness bar.
- **Graduation ranking**, highest-confidence first:
  1. Full `CellStyle` attributes (lever 3, minus `dim`) — cheapest,
     already SGR-coalesced, no structural risk. Recommend committing
     as-is via a real brainstorm, including the `Intensity` enum
     refactor flagged in Task 5 (folding `bold` and a proper `dim`
     into one tri-state field instead of independent bools).
  2. Sub-cell `Canvas` (lever 2) — both modes worked; recommend a real
     spec deciding whether `HalfBlock`/`Braille` stay one type with a
     mode enum (as prototyped) or split into two types.
  3. Gradient color ramps (lever 4) — `easing::lerp_color` already
     covers this; mainly needs a real widget-level home (e.g. a
     gradient option on `Block`/`Theme`), not new core math.
  4. Alpha blending (lever 5) — works for opaque-to-Rgb-target fades
     (as used for the particle trail) but **cannot gradually fade
     toward true transparency** (`Color::Reset`) — `lerp_color`'s
     non-Rgb fallback makes any real "fade to transparent" require an
     actual alpha channel on `Cell`, confirming this lever's
     flagged structural risk. Recommend a dedicated spec if pursued,
     given the `Cell`-shape cost.
  5. Particle trails (lever 6) — validated as an application of levers
     3/5/existing `ParticleSystem`, not a new primitive of its own;
     no separate Arc needed, it falls out of whichever of 1/4 lands.
```

Fill in the two `[fill in: ...]` placeholders with the actual Task 1
and Step 2 observations before committing — this document must not
ship with literal placeholder text left in.

- [ ] **Step 5: Commit**

```bash
git add examples/render_spike.rs \
  docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md
git commit -m "research(core): measure frame cost, record spike recommendations"
```

---

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo run --example render_spike` shows the full assembled scene
  from Task 8, animating smoothly, Space spawning fading particle
  bursts, `q` quitting cleanly.
- [ ] `cargo run --example render_spike -- --bench` prints frame-time
  numbers (already recorded in the spec by Task 9).
- [ ] `cargo build --examples` — all existing examples (`demo`,
  `omnitrix`, `tardis`, `smash_crabs`, `launcher`, `render_spike`)
  still compile, confirming the `CellStyle` field additions didn't
  break any existing call site.
- [ ] `cargo test` — full existing suite still green (this plan adds
  no new unit tests per the `research` TDD exemption, but must not
  break existing ones).
- [ ] The spec's "Recommendations (post-spike)" section is filled in
  with real findings, no placeholder text remaining.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this
  Arc's worktree branch to `main`, wait for the four required checks
  green, squash-merge, then remove the worktree via `ExitWorktree`.
