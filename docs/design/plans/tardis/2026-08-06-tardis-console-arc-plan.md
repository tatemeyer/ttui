# TARDIS Console + Boot + Artron Energy Arc Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-06-tardis-console-arc-design.md`:
a new `examples/tardis.rs` (greenfield — no such file exists yet) built
around a hexagonal, camera-panned console hub with perspective dimming,
a materialization boot sequence, a real Artron Energy sub-app, a
camera-flight transition between console faces, and looping + one-shot
procedural audio. Plus two new core-adjacent modules (`src/camera.rs`,
`src/glitch.rs`) and three new core widgets (`Roundel`, `AnalogToggle`,
`TimeRotor`).

**Architecture:** Ten tasks. Tasks 1-5 are core-framework (`src/`),
TDD-mandatory, independent of each other. Tasks 6-10 are all
`examples/tardis.rs`, strictly sequential, and are example code —
verified by running, not unit tested. Task order within 6-10 follows
the same "build the reachable-but-plain version first, add the big
transition once there's real content to preview" lesson learned in the
Smash Crabs arc: Hub with **instant** face-switching (6) → Artron
Energy's real content (7) → upgrade instant switching into the camera-
flight transition, now with real content to preview (8) → boot sequence
gating entry, its final phase reusing the by-then-complete Hub render
(9) → audio last, since its call sites live inside all four prior
tasks' handlers (10).

**Tech Stack:** Rust, `crossterm`, `rodio` 0.22 (already a
`[dev-dependencies]` entry from the Smash Crabs arc — no `Cargo.toml`
change needed in this plan).

## Global Constraints

- TDD mandatory for Tasks 1-5 (`coding`-tagged, no exception applies).
  Tasks 6-10 (`examples/tardis.rs`) are example code — per
  `.claude/rules/development-conventions.md`'s TDD exceptions, verified
  by running the example, not unit tested.
- Inline `#[cfg(test)] mod tests` per module — no new `tests/` directory.
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` clean after
  every task. Use `.is_multiple_of(2)` instead of `% 2 == 0` everywhere
  (clippy's `manual_is_multiple_of` lint, hit twice already in this
  project's history).
- No dependency changes to the `ttui` library itself (Tasks 1-5). No
  `Cargo.toml` change at all in this plan — `rodio` is already present.
- No RNG anywhere: `GlitchBuffer`'s noise, `TimeRotor`'s Braille pulse,
  and the flight transition's particle streaks are all deterministic
  hash-based, matching this project's established posture.
- `camera::dim()` and `Roundel`'s intensity scaling only affect
  `Color::Rgb` cells — `examples/tardis.rs`'s theme must use
  `Color::Rgb` throughout for anything that needs to visibly dim or
  pulse (same kind of hard constraint as `ScuttleCursor`'s single-width-
  glyph requirement in the Smash Crabs arc).
- The hexagonal console is a **navigation model**, not a rendered 3D
  shape — 6 logical faces laid out side by side in a virtual `Buffer`,
  panned via `camera::viewport`. No literal per-cell rotation (Rev B
  flagged this as likely permanently out of scope).

---

### Task 1: Camera + viewport + dim (`src/camera.rs`, #TBD)

**Files:**
- Create: `src/camera.rs`
- Modify: `src/lib.rs` (register the module)

**Interfaces produced:**
```rust
pub struct Camera { pub x: f32, pub y: f32, pub zoom: f32 }
impl Camera { pub fn new(x: f32, y: f32, zoom: f32) -> Self; }
pub fn viewport(source: &Buffer, camera: &Camera, width: u16, height: u16) -> Buffer;
pub fn dim(buf: &Buffer, factor: f32) -> Buffer;
```

- [ ] **Step 1: Write the failing tests** — create `src/camera.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crossterm::style::Color;

pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Camera {
    pub fn new(x: f32, y: f32, zoom: f32) -> Self {
        Camera { x, y, zoom }
    }
}

pub fn viewport(_source: &Buffer, _camera: &Camera, width: u16, height: u16) -> Buffer {
    let _ = (width, height);
    unimplemented!()
}

pub fn dim(_buf: &Buffer, _factor: f32) -> Buffer {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labeled(symbol: char) -> Cell {
        Cell {
            symbol,
            ..Default::default()
        }
    }

    #[test]
    fn viewport_at_zoom_one_crops_a_window_at_the_camera_position() {
        let mut source = Buffer::new(10, 10);
        source.set(3, 4, labeled('A'));
        source.set(4, 4, labeled('B'));
        let camera = Camera::new(3.0, 4.0, 1.0);

        let out = viewport(&source, &camera, 5, 3);

        assert_eq!(out.get(0, 0).symbol, 'A');
        assert_eq!(out.get(1, 0).symbol, 'B');
    }

    #[test]
    fn viewport_at_zoom_two_duplicates_each_source_cell_across_two_output_cells() {
        let mut source = Buffer::new(4, 1);
        source.set(0, 0, labeled('A'));
        source.set(1, 0, labeled('B'));
        let camera = Camera::new(0.0, 0.0, 2.0);

        let out = viewport(&source, &camera, 4, 1);

        assert_eq!(out.get(0, 0).symbol, 'A');
        assert_eq!(out.get(1, 0).symbol, 'A');
        assert_eq!(out.get(2, 0).symbol, 'B');
        assert_eq!(out.get(3, 0).symbol, 'B');
    }

    #[test]
    fn viewport_with_an_out_of_bounds_camera_does_not_panic() {
        let source = Buffer::new(4, 4);
        let camera = Camera::new(-5.0, -5.0, 1.0);

        let out = viewport(&source, &camera, 3, 3);

        assert_eq!(*out.get(0, 0), Cell::default());
    }

    #[test]
    fn dim_at_factor_zero_leaves_rgb_cells_unchanged() {
        let mut buf = Buffer::new(1, 1);
        buf.set(
            0,
            0,
            Cell {
                symbol: 'X',
                fg: Color::Rgb {
                    r: 100,
                    g: 150,
                    b: 200,
                },
                bg: Color::Rgb {
                    r: 10,
                    g: 20,
                    b: 30,
                },
                ..Default::default()
            },
        );

        let out = dim(&buf, 0.0);

        assert_eq!(
            out.get(0, 0).fg,
            Color::Rgb {
                r: 100,
                g: 150,
                b: 200
            }
        );
        assert_eq!(
            out.get(0, 0).bg,
            Color::Rgb {
                r: 10,
                g: 20,
                b: 30
            }
        );
        assert_eq!(out.get(0, 0).symbol, 'X');
    }

    #[test]
    fn dim_at_factor_one_drives_rgb_cells_to_black() {
        let mut buf = Buffer::new(1, 1);
        buf.set(
            0,
            0,
            Cell {
                fg: Color::Rgb {
                    r: 100,
                    g: 150,
                    b: 200,
                },
                bg: Color::Reset,
                ..Default::default()
            },
        );

        let out = dim(&buf, 1.0);

        assert_eq!(
            out.get(0, 0).fg,
            Color::Rgb { r: 0, g: 0, b: 0 }
        );
    }

    #[test]
    fn dim_at_factor_half_halves_each_channel() {
        let mut buf = Buffer::new(1, 1);
        buf.set(
            0,
            0,
            Cell {
                fg: Color::Rgb {
                    r: 100,
                    g: 150,
                    b: 200,
                },
                bg: Color::Reset,
                ..Default::default()
            },
        );

        let out = dim(&buf, 0.5);

        assert_eq!(
            out.get(0, 0).fg,
            Color::Rgb {
                r: 50,
                g: 75,
                b: 100
            }
        );
    }

    #[test]
    fn dim_leaves_non_rgb_colors_unaffected() {
        let mut buf = Buffer::new(1, 1);
        buf.set(
            0,
            0,
            Cell {
                fg: Color::Red,
                bg: Color::Reset,
                symbol: 'Y',
                ..Default::default()
            },
        );

        let out = dim(&buf, 1.0);

        assert_eq!(out.get(0, 0).fg, Color::Red);
        assert_eq!(out.get(0, 0).bg, Color::Reset);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib camera::tests`
Expected: all 7 FAIL (`not implemented`) — register the module first
(Step 5) so this compiles.

- [ ] **Step 3: Implement** — replace `viewport` and `dim`:

```rust
pub fn viewport(source: &Buffer, camera: &Camera, width: u16, height: u16) -> Buffer {
    let mut out = Buffer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let src_x = (camera.x + x as f32 / camera.zoom).floor();
            let src_y = (camera.y + y as f32 / camera.zoom).floor();
            if src_x >= 0.0
                && src_y >= 0.0
                && (src_x as u16) < source.width
                && (src_y as u16) < source.height
            {
                out.set(x, y, source.get(src_x as u16, src_y as u16).clone());
            }
        }
    }
    out
}

pub fn dim(buf: &Buffer, factor: f32) -> Buffer {
    let factor = factor.clamp(0.0, 1.0);
    let mut out = Buffer::new(buf.width, buf.height);
    for y in 0..buf.height {
        for x in 0..buf.width {
            let cell = buf.get(x, y);
            out.set(
                x,
                y,
                Cell {
                    fg: scale_color(cell.fg, factor),
                    bg: scale_color(cell.bg, factor),
                    ..cell.clone()
                },
            );
        }
    }
    out
}

fn scale_color(c: Color, factor: f32) -> Color {
    match c {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f32 * (1.0 - factor)) as u8,
            g: (g as f32 * (1.0 - factor)) as u8,
            b: (b as f32 * (1.0 - factor)) as u8,
        },
        other => other,
    }
}
```

  `viewport` uses `.floor()`, not `.round()` — this is what makes
  `zoom: 2.0` cleanly duplicate each source cell across exactly 2
  output cells (`floor(0/2)=0, floor(1/2)=0, floor(2/2)=1, floor(3/2)=1`),
  rather than rounding splitting a source cell's coverage unevenly.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib camera::tests`
Expected: all 7 PASS.

- [ ] **Step 5: Register the module** — add to `src/lib.rs` (find the
  existing `pub mod` list and insert alphabetically):

```rust
pub mod camera;
```

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/camera.rs src/lib.rs
git commit -m "feat(camera): add Camera, viewport, and dim helpers"
```

---

### Task 2: `GlitchBuffer` (`src/glitch.rs`, #TBD)

**Files:**
- Create: `src/glitch.rs`
- Modify: `src/lib.rs`

**Interfaces consumed:** `Transition::{start, tick, progress, is_complete}`
(`src/transition.rs`, unchanged).

**Interfaces produced:**
```rust
pub struct GlitchBuffer { /* private */ }
impl GlitchBuffer {
    pub fn new() -> Self;
    pub fn trigger(&mut self, duration: Duration);
    pub fn tick(&mut self, elapsed: Duration);
    pub fn is_active(&self) -> bool;
    pub fn render(&self, area: Rect, color: Color, tick_count: u64, buf: &mut Buffer);
}
```

- [ ] **Step 1: Write the failing tests** — create `src/glitch.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::transition::Transition;
use crossterm::style::Color;
use std::time::Duration;

const GLYPHS: [char; 4] = ['░', '▒', '▓', '█'];

pub struct GlitchBuffer {
    transition: Option<Transition>,
}

impl GlitchBuffer {
    pub fn new() -> Self {
        GlitchBuffer { transition: None }
    }

    pub fn trigger(&mut self, duration: Duration) {
        self.transition = Some(Transition::start(duration));
    }

    pub fn tick(&mut self, elapsed: Duration) {
        if let Some(t) = &mut self.transition {
            t.tick(elapsed);
            if t.is_complete() {
                self.transition = None;
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.transition.is_some()
    }

    pub fn render(&self, _area: Rect, _color: Color, _tick_count: u64, _buf: &mut Buffer) {
        unimplemented!()
    }
}

impl Default for GlitchBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        }
    }

    #[test]
    fn fresh_glitch_buffer_is_inactive_and_render_is_a_no_op() {
        let gb = GlitchBuffer::new();
        let mut buf = Buffer::new(3, 3);

        assert!(!gb.is_active());
        gb.render(area(), Color::Red, 0, &mut buf);

        assert_eq!(*buf.get(1, 1), Cell::default());
    }

    #[test]
    fn trigger_makes_is_active_true() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(500));
        assert!(gb.is_active());
    }

    #[test]
    fn ticking_past_the_triggered_duration_deactivates_it() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(500));
        gb.tick(Duration::from_millis(600));
        assert!(!gb.is_active());

        let mut buf = Buffer::new(3, 3);
        gb.render(area(), Color::Red, 0, &mut buf);
        assert_eq!(*buf.get(1, 1), Cell::default());
    }

    #[test]
    fn at_full_intensity_every_cell_is_glitched_with_the_requested_color() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(500));
        let mut buf = Buffer::new(3, 3);

        gb.render(area(), Color::Red, 0, &mut buf);

        for y in 0..3 {
            for x in 0..3 {
                let cell = buf.get(x, y);
                assert_ne!(*cell, Cell::default());
                assert_eq!(cell.fg, Color::Red);
                assert!(GLYPHS.contains(&cell.symbol));
            }
        }
    }
}
```

- [ ] **Step 2: Run tests to verify the render-dependent ones fail**

Run: `cargo test --lib glitch::tests`
Expected: `fresh_glitch_buffer_is_inactive_and_render_is_a_no_op`,
`ticking_past_the_triggered_duration_deactivates_it`, and
`at_full_intensity_every_cell_is_glitched_with_the_requested_color` FAIL
(`not implemented`); `trigger_makes_is_active_true` PASSES already
(doesn't call `render`).

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl GlitchBuffer {
    // ... new/trigger/tick/is_active unchanged ...

    pub fn render(&self, area: Rect, color: Color, tick_count: u64, buf: &mut Buffer) {
        let Some(t) = &self.transition else { return };
        let intensity = 1.0 - t.progress();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let h = (x as u64)
                    .wrapping_mul(374_761_393)
                    ^ (y as u64).wrapping_mul(668_265_263)
                    ^ tick_count.wrapping_mul(2_246_822_519);
                let roll = (h % 1000) as f32 / 1000.0;
                if roll < intensity {
                    let glyph = GLYPHS[(h / 1000 % 4) as usize];
                    buf.set(
                        x,
                        y,
                        Cell {
                            symbol: glyph,
                            fg: color,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib glitch::tests`
Expected: all 4 PASS.

- [ ] **Step 5: Register the module** — add to `src/lib.rs`:

```rust
pub mod glitch;
```

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/glitch.rs src/lib.rs
git commit -m "feat(glitch): add GlitchBuffer decaying noise overlay"
```

---

### Task 3: `Roundel` widget (`src/widgets/roundel.rs`, #TBD)

**Files:**
- Create: `src/widgets/roundel.rs`
- Modify: `src/widgets/mod.rs`

**Interfaces produced:**
```rust
pub struct Roundel { /* private */ }
impl Roundel {
    pub fn new(intensity: f32, color: Color) -> Self;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}
```

- [ ] **Step 1: Write the failing tests** — create `src/widgets/roundel.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct Roundel {
    intensity: f32,
    color: Color,
}

impl Roundel {
    pub fn new(intensity: f32, color: Color) -> Self {
        Roundel {
            intensity: intensity.clamp(0.0, 1.0),
            color,
        }
    }

    pub fn render(&self, _area: Rect, _buf: &mut Buffer) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area3x3() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        }
    }

    #[test]
    fn zero_intensity_renders_near_black() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
        )
        .render(area3x3(), &mut buf);

        assert_eq!(buf.get(1, 1).fg, Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn full_intensity_renders_the_input_color_unchanged() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            1.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
        )
        .render(area3x3(), &mut buf);

        assert_eq!(
            buf.get(1, 1).fg,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50
            }
        );
    }

    #[test]
    fn half_intensity_halves_each_channel() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.5,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
        )
        .render(area3x3(), &mut buf);

        assert_eq!(
            buf.get(1, 1).fg,
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
    }

    #[test]
    fn renders_at_area_center_and_does_not_panic_on_a_1x1_area() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };

        Roundel::new(1.0, Color::White).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'O');
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::roundel::tests`
Expected: all 4 FAIL (`not implemented`).

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl Roundel {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let cx = area.x + area.width / 2;
        let cy = area.y + area.height / 2;
        buf.set(
            cx,
            cy,
            Cell {
                symbol: 'O',
                fg: scale_color(self.color, self.intensity),
                bg: Color::Reset,
                ..Default::default()
            },
        );
    }
}

fn scale_color(c: Color, intensity: f32) -> Color {
    match c {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f32 * intensity) as u8,
            g: (g as f32 * intensity) as u8,
            b: (b as f32 * intensity) as u8,
        },
        other => other,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::roundel::tests`
Expected: all 4 PASS.

- [ ] **Step 5: Register the module** — add `pub mod roundel;` to
  `src/widgets/mod.rs` (alphabetically, alongside the others). Also add
  `pub mod analog_toggle;` and `pub mod time_rotor;` now if you intend
  to build Tasks 3-5 before running any check — otherwise add each as
  its own task adds its file.

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/roundel.rs src/widgets/mod.rs
git commit -m "feat(widgets): add Roundel widget"
```

---

### Task 4: `AnalogToggle` widget (`src/widgets/analog_toggle.rs`, #TBD)

**Files:**
- Create: `src/widgets/analog_toggle.rs`
- Modify: `src/widgets/mod.rs`

**Interfaces produced:**
```rust
pub struct AnalogToggle { /* private */ }
impl AnalogToggle {
    pub fn new(on: bool) -> Self;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}
```

- [ ] **Step 1: Write the failing tests** — create `src/widgets/analog_toggle.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;

pub struct AnalogToggle {
    on: bool,
}

impl AnalogToggle {
    pub fn new(on: bool) -> Self {
        AnalogToggle { on }
    }

    pub fn render(&self, _area: Rect, _buf: &mut Buffer) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area5x1() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 1,
        }
    }

    #[test]
    fn off_renders_a_backslash_lever() {
        let mut buf = Buffer::new(5, 1);
        AnalogToggle::new(false).render(area5x1(), &mut buf);

        let expected = ['[', ' ', '\\', ' ', ']'];
        for (i, ch) in expected.iter().enumerate() {
            assert_eq!(buf.get(i as u16, 0).symbol, *ch);
        }
    }

    #[test]
    fn on_renders_a_forward_slash_lever() {
        let mut buf = Buffer::new(5, 1);
        AnalogToggle::new(true).render(area5x1(), &mut buf);

        let expected = ['[', ' ', '/', ' ', ']'];
        for (i, ch) in expected.iter().enumerate() {
            assert_eq!(buf.get(i as u16, 0).symbol, *ch);
        }
    }

    #[test]
    fn narrower_than_five_clips_without_panic() {
        let mut buf = Buffer::new(2, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        };

        AnalogToggle::new(false).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '[');
        assert_eq!(buf.get(1, 0).symbol, ' ');
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::analog_toggle::tests`
Expected: all 3 FAIL (`not implemented`).

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl AnalogToggle {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let text = if self.on { "[ / ]" } else { "[ \\ ]" };
        for (i, ch) in text.chars().take(area.width as usize).enumerate() {
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol: ch,
                    ..Default::default()
                },
            );
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::analog_toggle::tests`
Expected: all 3 PASS.

- [ ] **Step 5: Register the module** — confirm `pub mod analog_toggle;`
  is in `src/widgets/mod.rs`.

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/analog_toggle.rs src/widgets/mod.rs
git commit -m "feat(widgets): add AnalogToggle widget"
```

---

### Task 5: `TimeRotor` widget (`src/widgets/time_rotor.rs`, #TBD)

**Files:**
- Create: `src/widgets/time_rotor.rs`
- Modify: `src/widgets/mod.rs`

**Interfaces produced:**
```rust
pub struct TimeRotor { /* private */ }
impl TimeRotor {
    pub fn new(speed: f32) -> Self;
    pub fn render(&self, area: Rect, tick_count: u64, buf: &mut Buffer);
}
```

- [ ] **Step 1: Write the failing tests** — create `src/widgets/time_rotor.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;

pub struct TimeRotor {
    speed: f32,
}

impl TimeRotor {
    pub fn new(speed: f32) -> Self {
        TimeRotor {
            speed: speed.max(0.1),
        }
    }

    pub fn render(&self, _area: Rect, _tick_count: u64, _buf: &mut Buffer) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 4,
        }
    }

    fn is_braille(ch: char) -> bool {
        ('\u{2800}'..='\u{28FF}').contains(&ch)
    }

    #[test]
    fn renders_one_braille_glyph_per_row_at_the_center_column() {
        let mut buf = Buffer::new(5, 4);
        TimeRotor::new(1.0).render(area(), 0, &mut buf);

        for row in 0..4 {
            assert!(is_braille(buf.get(2, row).symbol));
        }
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn identical_inputs_render_identically() {
        let mut buf_a = Buffer::new(5, 4);
        let mut buf_b = Buffer::new(5, 4);
        TimeRotor::new(2.5).render(area(), 7, &mut buf_a);
        TimeRotor::new(2.5).render(area(), 7, &mut buf_b);

        for row in 0..4 {
            assert_eq!(buf_a.get(2, row), buf_b.get(2, row));
        }
    }

    #[test]
    fn different_speeds_render_differently_for_the_same_tick_count() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 2,
        };
        let mut slow = Buffer::new(3, 2);
        let mut fast = Buffer::new(3, 2);

        TimeRotor::new(1.0).render(area, 10, &mut slow);
        TimeRotor::new(5.0).render(area, 10, &mut fast);

        // Hand-verified for these exact inputs (area width 3, tick 10):
        // row 0's hash differs between speed 1.0 (scaled_tick=10) and
        // speed 5.0 (scaled_tick=50), so the two buffers are not equal.
        assert_ne!(slow, fast);
    }
}
```

  Note: this requires `Buffer` to implement `PartialEq` (already does,
  per `src/buffer.rs`'s existing `#[derive(Clone, PartialEq, Debug)]`
  on `Cell` and the buffer comparison used implicitly via `get()`
  equality elsewhere — if `Buffer` itself isn't already `#[derive(PartialEq)]`,
  compare `buf.get(x, y)` cell-by-cell across the 3x2 area instead of
  comparing the whole `Buffer` directly).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::time_rotor::tests`
Expected: the two render-dependent tests FAIL (`not implemented`);
`identical_inputs_render_identically` also FAILS since it calls
`render` twice.

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl TimeRotor {
    pub fn render(&self, area: Rect, tick_count: u64, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let cx = area.x + area.width / 2;
        let scaled_tick = (tick_count as f32 * self.speed) as u64;
        for row in 0..area.height {
            let h = (row as u64).wrapping_mul(374_761_393)
                ^ scaled_tick.wrapping_mul(668_265_263);
            let dot_pattern = (h % 256) as u32;
            let glyph = char::from_u32(0x2800 + dot_pattern).unwrap_or('\u{2800}');
            buf.set(
                cx,
                area.y + row,
                Cell {
                    symbol: glyph,
                    ..Default::default()
                },
            );
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::time_rotor::tests`
Expected: all 3 PASS. If `different_speeds_render_differently_for_the_
same_tick_count` fails because `Buffer` doesn't implement `PartialEq`,
rewrite that assertion as a per-cell loop comparing `slow.get(x, y)` vs
`fast.get(x, y)` and asserting at least one pair differs, rather than
adding a `PartialEq` derive to `Buffer` (out of scope for this task).

- [ ] **Step 5: Register the module** — confirm `pub mod time_rotor;`
  is in `src/widgets/mod.rs`, and that it (plus `roundel`/
  `analog_toggle` from Tasks 3-4) are all present:

```rust
pub mod analog_toggle;
pub mod block;
pub mod damage_meter;
pub mod dial;
pub mod list;
pub mod roundel;
pub mod scuttle_cursor;
pub mod smash_border;
pub mod table;
pub mod text;
pub mod time_rotor;
```

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/time_rotor.rs src/widgets/mod.rs
git commit -m "feat(widgets): add TimeRotor widget"
```

---

### Task 6: Hub skeleton with instant face switching (`examples/tardis.rs`, new file)

**Files:**
- Create: `examples/tardis.rs`

**Interfaces consumed:** `ttui::camera::{Camera, viewport, dim}` (Task 1);
`ttui::widgets::{roundel::Roundel, time_rotor::TimeRotor}` (Tasks 3, 5);
`ttui::easing::ease_out`; `ttui::transition::Transition`; `ttui::app::{run, App}`.

**Interfaces produced:** none public — everything is private to this
new example binary. Internal shape later tasks build on: `Screen`,
`FACE_COUNT`, `FACE_NAMES`, `screen_for_face`, `hex_distance`, `Tardis`
struct + `new()`, `render_hub`, `render_placeholder`, `blit`,
`time_rotor_speed` (stubbed to a constant here, real logic in Task 7).

No new tests — example code, verified by running.

- [ ] **Step 1: Write the whole file** — create `examples/tardis.rs`:

```rust
// examples/tardis.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Buffer, Cell, LayerStack};
use ttui::camera::{self, Camera};
use ttui::easing;
use ttui::layout::Rect;
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{roundel::Roundel, text::Text, time_rotor::TimeRotor};

const TICK_INTERVAL: Duration = Duration::from_millis(33);
const FACE_COUNT: usize = 6;
const FACE_NAMES: [&str; 6] = [
    "Psychic Paper",
    "Auxiliary Roundel Bay",
    "Star Charts",
    "Auxiliary Roundel Bay",
    "Artron Energy",
    "Auxiliary Roundel Bay",
];
const ROTATE_TWEEN_MS: u64 = 200;
const DIM_FACTORS: [f32; 4] = [0.0, 0.35, 0.65, 0.85];

fn tardis_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 0, g: 0, b: 0 },
        primary: Color::Rgb { r: 0, g: 255, b: 20 },
        secondary: Color::Rgb {
            r: 184,
            g: 115,
            b: 51,
        },
        tertiary: Color::Rgb {
            r: 0,
            g: 255,
            b: 255,
        },
        accent: Color::Rgb {
            r: 255,
            g: 191,
            b: 0,
        },
        border: BorderSet {
            horizontal: '=',
            vertical: '#',
            corner: '+',
        },
        border_bold: false,
        border_thick: false,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Hub,
    PsychicPaper,
    StarCharts,
    ArtronEnergy,
}

fn screen_for_face(face: usize) -> Option<Screen> {
    match face {
        0 => Some(Screen::PsychicPaper),
        2 => Some(Screen::StarCharts),
        4 => Some(Screen::ArtronEnergy),
        _ => None,
    }
}

fn hex_distance(a: usize, b: usize) -> usize {
    let diff = if a > b { a - b } else { b - a };
    diff.min(FACE_COUNT - diff)
}

struct Tardis {
    theme: Theme,
    screen: Screen,
    selected_face: usize,
    face_tween: Option<(f32, Transition)>,
    tick_count: u64,
    quit: bool,
}

impl Tardis {
    fn new() -> Self {
        Tardis {
            theme: tardis_theme(),
            screen: Screen::Hub,
            selected_face: 0,
            face_tween: None,
            tick_count: 0,
            quit: false,
        }
    }

    fn displayed_face_index(&self) -> f32 {
        match &self.face_tween {
            Some((from, t)) => easing::ease_out(*from, self.selected_face as f32, t.progress()),
            None => self.selected_face as f32,
        }
    }

    fn time_rotor_speed(&self) -> f32 {
        1.0
    }

    fn render_face_content(&self, face: usize, area: Rect, buf: &mut Buffer) {
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        Text::new(FACE_NAMES[face]).render(name_row, buf);
        if screen_for_face(face).is_none() {
            for i in 0..3u16 {
                let rx = area.x + (area.width / 4) * (i + 1);
                let ry = area.y + area.height / 2;
                let pulse = ((self.tick_count as f32 * 0.05 + i as f32).sin() + 1.0) / 2.0;
                Roundel::new(pulse, self.theme.tertiary).render(
                    Rect {
                        x: rx,
                        y: ry,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
            }
        }
    }

    fn render_hub(&self, area: Rect, buf: &mut LayerStack) {
        let vw = area.width;
        let vh = area.height;
        let mut virtual_buf = Buffer::new(vw * FACE_COUNT as u16, vh);
        for face in 0..FACE_COUNT {
            let face_area = Rect {
                x: face as u16 * vw,
                y: 0,
                width: vw,
                height: vh,
            };
            self.render_face_content(face, face_area, &mut virtual_buf);
            let factor = DIM_FACTORS[hex_distance(face, self.selected_face)];
            if factor > 0.0 {
                let face_camera = Camera::new(face_area.x as f32, face_area.y as f32, 1.0);
                let cropped = camera::viewport(&virtual_buf, &face_camera, vw, vh);
                let dimmed = camera::dim(&cropped, factor);
                blit(&dimmed, face_area, &mut virtual_buf);
            }
        }
        let cam = Camera::new(self.displayed_face_index() * vw as f32, 0.0, 1.0);
        let view = camera::viewport(&virtual_buf, &cam, vw, vh);
        blit(&view, area, buf);

        let rotor_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        TimeRotor::new(self.time_rotor_speed()).render(rotor_area, self.tick_count, buf);

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Left/Right rotate * Enter select * q quit").render(hint_row, buf);
    }

    fn render_placeholder(&self, screen: Screen, area: Rect, buf: &mut LayerStack) {
        let name = match screen {
            Screen::PsychicPaper => "Psychic Paper",
            Screen::StarCharts => "Star Charts",
            Screen::ArtronEnergy => "Artron Energy",
            Screen::Hub => "",
        };
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        let placeholder_row = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new(name).render(name_row, buf);
        Text::new("(not yet built)").render(placeholder_row, buf);
        Text::new("Esc back * q quit").render(hint_row, buf);
    }
}

fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}

impl App for Tardis {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        match self.screen {
            Screen::Hub => match k.code {
                KeyCode::Left => {
                    let from = self.displayed_face_index();
                    self.selected_face = (self.selected_face + FACE_COUNT - 1) % FACE_COUNT;
                    self.face_tween =
                        Some((from, Transition::start(Duration::from_millis(ROTATE_TWEEN_MS))));
                }
                KeyCode::Right => {
                    let from = self.displayed_face_index();
                    self.selected_face = (self.selected_face + 1) % FACE_COUNT;
                    self.face_tween =
                        Some((from, Transition::start(Duration::from_millis(ROTATE_TWEEN_MS))));
                }
                KeyCode::Enter => {
                    if self.face_tween.is_none() {
                        if let Some(dest) = screen_for_face(self.selected_face) {
                            self.screen = dest;
                        }
                    }
                }
                _ => {}
            },
            _ => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        match self.screen {
            Screen::Hub => self.render_hub(area, buf),
            Screen::PsychicPaper | Screen::StarCharts | Screen::ArtronEnergy => {
                self.render_placeholder(self.screen, area, buf)
            }
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, elapsed: Duration) {
        self.tick_count += 1;
        if let Some((_, t)) = &mut self.face_tween {
            t.tick(elapsed);
            if t.is_complete() {
                self.face_tween = None;
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut app = Tardis::new();
    run(&mut app)
}
```

- [ ] **Step 2: Build**

Run: `cargo build --example tardis`
Expected: compiles cleanly, no warnings.

- [ ] **Step 3: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 4: Manual verification** (real-terminal check, not
  automatable — per this project's TDD exceptions for example code):

Run: `cargo run --example tardis`

Confirm:
- App opens directly on the Hub (no boot sequence yet — Task 9 adds
  that) showing "Psychic Paper" with a pulsing `TimeRotor` column
  behind it.
- `Right`/`Left` pan smoothly between all 6 faces with visible dimming
  of the view as you move away from a face and brightening as you
  arrive — including the 3 "Auxiliary Roundel Bay" decorative faces
  with their ambient pulsing `Roundel`s.
- `Enter` on Psychic Paper/Star Charts/Artron Energy switches
  **instantly** (no transition yet) to that screen's placeholder;
  `Enter` on an Auxiliary Roundel Bay face does nothing.
- `Esc` from any placeholder returns to the Hub with the same face
  still selected.
- `q` quits cleanly from every state, no panic, no leftover terminal
  attributes.

- [ ] **Step 5: Commit**

```bash
git add examples/tardis.rs
git commit -m "feat(tardis): add hexagonal console Hub with instant face switching"
```

---

### Task 7: Artron Energy sub-app (`examples/tardis.rs`)

**Files:**
- Modify: `examples/tardis.rs`

**Interfaces consumed:** `ttui::glitch::GlitchBuffer` (Task 2);
`ttui::widgets::analog_toggle::AnalogToggle` (Task 4); `ttui::particles::
{Particle, ParticleSystem}` (Arc 0, unchanged).

No new tests — example code, verified by running.

- [ ] **Step 1: Update imports**:

```rust
use ttui::glitch::GlitchBuffer;
use ttui::particles::{Particle, ParticleSystem};
use ttui::widgets::{
    analog_toggle::AnalogToggle, roundel::Roundel, text::Text, time_rotor::TimeRotor,
};
```

- [ ] **Step 2: Add the tuning constants** — alongside `DIM_FACTORS`:

```rust
const ENERGY_GAIN_PER_HIT: f32 = 12.0;
const ENERGY_VENT_AMOUNT: f32 = 35.0;
const ENERGY_DECAY_PER_SEC: f32 = 4.0;
const VENT_FLASH_MS: u64 = 300;
const VENTING_THRESHOLD: f32 = 80.0;
const LAG_THRESHOLD: f32 = 90.0;
const GLITCH_DURATION_MS: u64 = 500;
const LAGGING_TICK_INTERVAL: Duration = Duration::from_millis(66);
```

- [ ] **Step 3: Add fields** — change the struct definition (insert
  before `tick_count`):

```rust
struct Tardis {
    theme: Theme,
    screen: Screen,
    selected_face: usize,
    face_tween: Option<(f32, Transition)>,
    energy: f32,
    vent_flash: Option<Transition>,
    glitch: GlitchBuffer,
    particles: ParticleSystem,
    tick_count: u64,
    quit: bool,
}
```

  and in `new()`:

```rust
            face_tween: None,
            energy: 0.0,
            vent_flash: None,
            glitch: GlitchBuffer::new(),
            particles: ParticleSystem::new(),
            tick_count: 0,
```

- [ ] **Step 4: Wire `energy` into `time_rotor_speed` and add
  `is_lagging`** — replace:

```rust
    fn time_rotor_speed(&self) -> f32 {
        1.0
    }
```

  with:

```rust
    fn time_rotor_speed(&self) -> f32 {
        1.0 + self.energy / 50.0
    }

    fn is_lagging(&self) -> bool {
        self.energy >= LAG_THRESHOLD
    }
```

- [ ] **Step 5: Add `render_artron_energy`** — add to `impl Tardis`:

```rust
    fn render_artron_energy(&self, area: Rect, buf: &mut LayerStack) {
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        Text::new("Artron Energy").render(name_row, buf);

        for i in 0..3u16 {
            let seg_intensity = ((self.energy - i as f32 * 33.0) / 33.0).clamp(0.0, 1.0);
            let rx = area.x + 4 + i * 4;
            let ry = area.y + 2;
            Roundel::new(seg_intensity, self.theme.tertiary).render(
                Rect {
                    x: rx,
                    y: ry,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }

        let toggle_row = Rect {
            x: area.x,
            y: area.y + 4,
            width: area.width.min(10),
            height: 1,
        };
        AnalogToggle::new(self.vent_flash.is_some()).render(toggle_row, buf);

        let rotor_area = Rect {
            x: area.x,
            y: area.y + 6,
            width: area.width,
            height: area.height.saturating_sub(8),
        };
        TimeRotor::new(self.time_rotor_speed()).render(rotor_area, self.tick_count, buf);

        if self.glitch.is_active() {
            self.glitch.render(area, Color::Red, self.tick_count, buf);
        }

        self.particles.render(buf);

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Space channel * v vent * Esc back * q quit").render(hint_row, buf);
    }
```

  `self.particles.render(buf)` writes at each particle's stored
  absolute `(x, y)`; since `view()`'s top-level `area` always starts at
  `(0, 0)` (per `app::run()`'s loop), particles spawned with small
  absolute coordinates land correctly without needing a scratch-buffer
  detour — unlike Smash Crabs, this screen has no shake post-process
  step forcing that indirection.

- [ ] **Step 6: Replace the Hub's `Enter`-into-ArtronEnergy and add the
  in-screen key handling** — change `view()`'s dispatch:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        match self.screen {
            Screen::Hub => self.render_hub(area, buf),
            Screen::ArtronEnergy => self.render_artron_energy(area, buf),
            Screen::PsychicPaper | Screen::StarCharts => {
                self.render_placeholder(self.screen, area, buf)
            }
        }
    }
```

  and replace `update()`'s catch-all non-Hub arm with per-screen
  handling:

```rust
            Screen::ArtronEnergy => match k.code {
                KeyCode::Esc => self.screen = Screen::Hub,
                KeyCode::Char(' ') => {
                    self.energy += ENERGY_GAIN_PER_HIT;
                    if self.energy >= VENTING_THRESHOLD {
                        for i in 0..8 {
                            let angle = i as f32 * std::f32::consts::TAU / 8.0;
                            self.particles.spawn(Particle {
                                x: 10.0,
                                y: 4.0,
                                vx: angle.cos() * 10.0,
                                vy: angle.sin() * 5.0,
                                symbol: '*',
                                color: Color::Red,
                                lifetime: Duration::from_millis(500),
                                age: Duration::ZERO,
                            });
                        }
                    }
                }
                KeyCode::Char('v') => {
                    self.energy = (self.energy - ENERGY_VENT_AMOUNT).max(0.0);
                    self.vent_flash =
                        Some(Transition::start(Duration::from_millis(VENT_FLASH_MS)));
                }
                _ => {}
            },
            Screen::PsychicPaper | Screen::StarCharts => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
```

  (this replaces the previous single `_ => { if k.code == KeyCode::Esc
  ... } }` catch-all arm — `Screen::Hub`'s arm above it is unchanged.)

- [ ] **Step 7: Tick energy decay, the glitch trigger, `vent_flash`, and
  particles** — append to the end of `on_tick` (after the existing
  `face_tween` block, before the closing brace):

```rust
        self.energy = (self.energy - ENERGY_DECAY_PER_SEC * elapsed.as_secs_f32()).max(0.0);

        if self.is_lagging() {
            self.glitch.trigger(Duration::from_millis(GLITCH_DURATION_MS));
        }
        self.glitch.tick(elapsed);

        if let Some(t) = &mut self.vent_flash {
            t.tick(elapsed);
            if t.is_complete() {
                self.vent_flash = None;
            }
        }

        self.particles.update(elapsed);
```

- [ ] **Step 8: Make `tick_rate` respond to lag** — replace:

```rust
    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }
```

  with:

```rust
    fn tick_rate(&self) -> Option<Duration> {
        if self.is_lagging() {
            Some(LAGGING_TICK_INTERVAL)
        } else {
            Some(TICK_INTERVAL)
        }
    }
```

- [ ] **Step 9: Build**

Run: `cargo build --example tardis`
Expected: compiles cleanly.

- [ ] **Step 10: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 11: Manual verification**

Run: `cargo run --example tardis`

Navigate (`Right`/`Right`, `Right`, `Right` from the start, or however
many presses land on face `4`) to Artron Energy and confirm:
- Three `Roundel` "pipe" segments light up left-to-right as you hold
  Space, each one visibly brighter than the last as `energy` climbs.
- Past `80`, hitting Space spawns a red particle burst.
- Past `90`, a `GlitchBuffer` overlay appears and the whole app
  visibly ticks slower (try rotating the console right after — the pan
  should feel noticeably laggier).
- `v` instantly drops energy and flips the `AnalogToggle` to its `/`
  position briefly.
- Back in the Hub, `TimeRotor`'s pulse speed reflects whatever `energy`
  was left at when you pressed `Esc` (it doesn't reset).
- `q` quits cleanly from Artron Energy too.

- [ ] **Step 12: Commit**

```bash
git add examples/tardis.rs
git commit -m "feat(tardis): add Artron Energy sub-app"
```

---

### Task 8: Flight transition (`examples/tardis.rs`)

**Files:**
- Modify: `examples/tardis.rs`

**Interfaces consumed:** `render_hub`, `render_artron_energy`,
`render_placeholder`, `blit` (all from Tasks 6-7, now needed to build a
destination-screen preview); `ttui::effects::shake` (Arc 0, unchanged).

No new tests — example code, verified by running.

- [ ] **Step 1: Add the `effects` import**:

```rust
use ttui::effects;
```

- [ ] **Step 2: Add transition state** — change the struct definition
  (insert after `particles`):

```rust
    particles: ParticleSystem,
    transitioning_to: Option<(Screen, Transition)>,
    tick_count: u64,
    quit: bool,
}
```

  and in `new()`:

```rust
            particles: ParticleSystem::new(),
            transitioning_to: None,
            tick_count: 0,
```

  Add the constant near `ROTATE_TWEEN_MS`:

```rust
const FLIGHT_TRANSITION_MS: u64 = 900;
```

- [ ] **Step 3: Replace the Hub's instant `Enter` switch with a
  transition start** — in `update()`'s `Screen::Hub` arm, replace:

```rust
                KeyCode::Enter => {
                    if self.face_tween.is_none() {
                        if let Some(dest) = screen_for_face(self.selected_face) {
                            self.screen = dest;
                        }
                    }
                }
```

  with:

```rust
                KeyCode::Enter => {
                    if self.face_tween.is_none() {
                        if let Some(dest) = screen_for_face(self.selected_face) {
                            self.transitioning_to = Some((
                                dest,
                                Transition::start(Duration::from_millis(FLIGHT_TRANSITION_MS)),
                            ));
                        }
                    }
                }
```

- [ ] **Step 4: Ignore navigation input while transitioning** — add a
  guard in `update()` right after the `q` check:

```rust
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        if self.transitioning_to.is_some() {
            return;
        }
        match self.screen {
```

- [ ] **Step 5: Tick the transition and land on completion** — append
  to `on_tick` (after the `particles.update` line, before the closing
  brace):

```rust
        if let Some((destination, t)) = &mut self.transitioning_to {
            t.tick(elapsed);
            if t.is_complete() {
                self.screen = *destination;
                self.transitioning_to = None;
            }
        }
```

- [ ] **Step 6: Add the destination-preview and transition-rendering
  helpers** — add to `impl Tardis`:

```rust
    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let mut stack = LayerStack::new(area.width, area.height);
        match screen {
            Screen::ArtronEnergy => self.render_artron_energy(local, &mut stack),
            Screen::PsychicPaper | Screen::StarCharts => {
                self.render_placeholder(screen, local, &mut stack)
            }
            Screen::Hub => self.render_hub(local, &mut stack),
        }
        let mut out = Buffer::new(area.width, area.height);
        blit(&stack, local, &mut out);
        out
    }

    fn render_transition(&self, destination: Screen, area: Rect, progress: f32, buf: &mut Buffer) {
        if progress < 0.3 {
            let local = Rect {
                x: 0,
                y: 0,
                width: area.width,
                height: area.height,
            };
            let mut stack = LayerStack::new(area.width, area.height);
            self.render_hub(local, &mut stack);
            let magnitude: i16 = 1 + (progress / 0.3 * 2.0) as i16;
            let dx = if self.tick_count.is_multiple_of(2) {
                magnitude
            } else {
                -magnitude
            };
            let dy = if (self.tick_count / 2).is_multiple_of(2) {
                magnitude
            } else {
                -magnitude
            };
            let shaken = effects::shake(&stack, dx, dy);
            blit(&shaken, area, buf);
            return;
        }

        if progress < 0.85 {
            for y in 0..area.height {
                for x in 0..area.width {
                    buf.set(
                        area.x + x,
                        area.y + y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: Color::Rgb { r: 5, g: 0, b: 15 },
                            ..Default::default()
                        },
                    );
                }
            }
            let void_progress = ((progress - 0.3) / 0.4).clamp(0.0, 1.0);
            let count = (void_progress * 20.0) as usize;
            let cx = area.width as f32 / 2.0;
            let cy = area.height as f32 / 2.0;
            let max_dist = cx.max(cy);
            for i in 0..count {
                let angle = i as f32 * std::f32::consts::TAU / 20.0;
                let dist = void_progress * max_dist;
                let x = (cx + angle.cos() * dist).round();
                let y = (cy + angle.sin() * dist * 0.5).round();
                if x >= 0.0 && y >= 0.0 && (x as u16) < area.width && (y as u16) < area.height {
                    buf.set(
                        area.x + x as u16,
                        area.y + y as u16,
                        Cell {
                            symbol: '-',
                            fg: Color::Rgb {
                                r: 0,
                                g: 255,
                                b: 255,
                            },
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
            return;
        }

        let content = self.render_destination_preview(destination, area);
        blit(&content, area, buf);
    }
```

  `render_destination_preview`'s `Hub` arm is included for completeness
  (`Screen` isn't exhaustively used as a transition *destination* — the
  flight transition only ever targets `PsychicPaper`/`StarCharts`/
  `ArtronEnergy` — but `screen: Screen` is a 4-variant type, so any
  `match` over it must handle `Hub` somewhere; this mirrors the same
  choice made in both prior arcs' analogous preview helpers).

- [ ] **Step 7: Branch `view()` on transition state** — replace the
  method body:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        if let Some((destination, transition)) = &self.transitioning_to {
            self.render_transition(*destination, area, transition.progress(), buf);
            return;
        }
        match self.screen {
            Screen::Hub => self.render_hub(area, buf),
            Screen::ArtronEnergy => self.render_artron_energy(area, buf),
            Screen::PsychicPaper | Screen::StarCharts => {
                self.render_placeholder(self.screen, area, buf)
            }
        }
    }
```

- [ ] **Step 8: Build**

Run: `cargo build --example tardis`
Expected: compiles cleanly.

- [ ] **Step 9: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 10: Manual verification**

Run: `cargo run --example tardis`

Confirm, selecting each of the 3 real faces in turn:
- `Enter` now plays a ~900ms flight: the Hub view visibly shakes and
  blurs (increasing jitter), then cuts to a dark void with cyan `-`
  streaks radiating outward and thickening, then hard-cuts into the
  destination screen already fully rendered.
- Navigation is entirely ignored while the flight plays; `q` still
  quits immediately even mid-flight.
- `Esc` from any destination still returns to the Hub instantly (no
  flight — unchanged from Task 6/7).
- `q` quits cleanly from every state.

- [ ] **Step 11: Commit**

```bash
git add examples/tardis.rs
git commit -m "feat(tardis): add camera-flight transition into console faces"
```

---

### Task 9: Boot sequence (`examples/tardis.rs`)

**Files:**
- Modify: `examples/tardis.rs`

**Interfaces consumed:** `render_hub` (Task 6, now reused by the
push-through phase); `camera::{Camera, viewport}` (Task 1);
`easing::ease_out` (Arc 0).

No new tests — example code, verified by running.

- [ ] **Step 1: Add the boot duration constant and ASCII art** — add
  near the other constants:

```rust
const BOOT_MS: u64 = 3000;

const POLICE_BOX_CLOSED: [&str; 5] = [
    "+------+",
    "|POLICE|",
    "|BOX   |",
    "|[DOOR]|",
    "+------+",
];
const POLICE_BOX_OPEN: [&str; 5] = [
    "+------+",
    "|POLICE|",
    "|BOX   |",
    "|[    ]|",
    "+------+",
];
```

- [ ] **Step 2: Add the `booting` field** — change the struct
  definition (insert after `transitioning_to`):

```rust
    transitioning_to: Option<(Screen, Transition)>,
    booting: Option<Transition>,
    tick_count: u64,
    quit: bool,
}
```

  and in `new()`:

```rust
            transitioning_to: None,
            booting: Some(Transition::start(Duration::from_millis(BOOT_MS))),
            tick_count: 0,
```

- [ ] **Step 3: Add `render_police_box` and `render_boot`** — add to
  `impl Tardis`:

```rust
    fn render_police_box(&self, area: Rect, lines: &[&str; 5], dx: i16, dy: i16, buf: &mut LayerStack) {
        let box_width: i32 = 8;
        let box_height: i32 = 5;
        let x0 = area.x as i32 + (area.width as i32 - box_width) / 2 + dx as i32;
        let y0 = area.y as i32 + (area.height as i32 - box_height) / 2 + dy as i32;
        for (row, line) in lines.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let px = x0 + col as i32;
                let py = y0 + row as i32;
                if px >= area.x as i32
                    && py >= area.y as i32
                    && (px as u16) < area.x + area.width
                    && (py as u16) < area.y + area.height
                {
                    buf.set(
                        px as u16,
                        py as u16,
                        Cell {
                            symbol: ch,
                            fg: self.theme.tertiary,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

    fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: Color::Black,
                        ..Default::default()
                    },
                );
            }
        }

        if progress < 0.15 {
            self.render_police_box(area, &POLICE_BOX_CLOSED, 0, 0, buf);
            return;
        }
        if progress < 0.35 {
            let magnitude: i16 = 2;
            let dx = if self.tick_count.is_multiple_of(2) {
                magnitude
            } else {
                -magnitude
            };
            let dy = if (self.tick_count / 2).is_multiple_of(2) {
                magnitude
            } else {
                -magnitude
            };
            self.render_police_box(area, &POLICE_BOX_CLOSED, dx, dy, buf);
            return;
        }
        if progress < 0.5 {
            self.render_police_box(area, &POLICE_BOX_OPEN, 0, 0, buf);
            return;
        }
        if progress < 0.65 {
            for y in 0..area.height {
                for x in 0..area.width {
                    buf.set(
                        area.x + x,
                        area.y + y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: Color::Rgb {
                                r: 255,
                                g: 255,
                                b: 255,
                            },
                            ..Default::default()
                        },
                    );
                }
            }
            return;
        }

        let push_progress = ((progress - 0.65) / 0.35).clamp(0.0, 1.0);
        let zoom = easing::ease_out(1.0, 2.2, push_progress);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let mut hub_stack = LayerStack::new(area.width, area.height);
        self.render_hub(local, &mut hub_stack);
        let cam = Camera::new(
            area.width as f32 / 2.0 * (1.0 - 1.0 / zoom),
            area.height as f32 / 2.0 * (1.0 - 1.0 / zoom),
            zoom,
        );
        let zoomed = camera::viewport(&hub_stack, &cam, area.width, area.height);
        blit(&zoomed, area, buf);
    }
```

- [ ] **Step 4: Gate `update()` on `booting`** — add a guard right at
  the top, before the `q` check stays first but the screen-dispatch
  guard moves after it:

```rust
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        if self.booting.is_some() {
            return;
        }
        if self.transitioning_to.is_some() {
            return;
        }
        match self.screen {
```

- [ ] **Step 5: Check `booting` first in `view()`**:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        if let Some((destination, transition)) = &self.transitioning_to {
```

  (the rest of `view()` is unchanged — this just adds one more early
  branch above the existing `transitioning_to` check.)

- [ ] **Step 6: Tick and clear `booting`** — append to `on_tick`
  (after the `transitioning_to` block, before the closing brace):

```rust
        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            if t.is_complete() {
                self.booting = None;
            }
        }
```

- [ ] **Step 7: Build**

Run: `cargo build --example tardis`
Expected: compiles cleanly.

- [ ] **Step 8: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 9: Manual verification**

Run: `cargo run --example tardis`

Confirm the full ~3-second boot plays automatically on startup before
anything is interactive: a small Police Box on black, then it jitters
in place, then its door glyph opens, then the screen flashes white,
then the view zooms/pushes into the already-familiar Hub. Confirm `q`
quits cleanly even mid-boot. Confirm once boot completes, Left/Right/
Enter/Esc all behave exactly as in Tasks 6-8 (this task changes nothing
about post-boot interaction).

- [ ] **Step 10: Commit**

```bash
git add examples/tardis.rs
git commit -m "feat(tardis): add materialization boot sequence"
```

---

### Task 10: Audio (`examples/tardis.rs`)

**Files:**
- Modify: `examples/tardis.rs`

**Interfaces consumed:** `ttui::audio::AudioSink` (`src/audio.rs`,
unchanged); `rodio::stream::{DeviceSinkBuilder, MixerDeviceSink}`,
`rodio::source::{SineWave, Source}` — same rodio 0.22 surface already
proven working in the Smash Crabs arc, plus `Source::repeat_infinite`
(confirmed against docs.rs during this arc's design — buffers a finite,
`take_duration`'d source and loops it, which is exactly the intended
use against `SineWave`'s otherwise-infinite signal).

**Interfaces produced:** none public.

No new tests — example code, verified by running; the looping hum and
one-shot cues cannot be verified by ear in this environment (no audio
device) — same caveat as the Smash Crabs arc.

- [ ] **Step 1: Add the imports**:

```rust
use rodio::Source;
use ttui::audio::AudioSink;
```

- [ ] **Step 2: Add `RodioAudioSink`** — add above `struct Tardis`:

```rust
struct RodioAudioSink {
    sink: Option<rodio::stream::MixerDeviceSink>,
}

impl RodioAudioSink {
    fn new() -> Self {
        match rodio::stream::DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => {
                let hum = rodio::source::SineWave::new(80.0)
                    .take_duration(Duration::from_secs(2))
                    .amplify(0.05)
                    .repeat_infinite();
                sink.mixer().add(hum);
                RodioAudioSink { sink: Some(sink) }
            }
            Err(_) => RodioAudioSink { sink: None },
        }
    }
}

impl AudioSink for RodioAudioSink {
    fn play(&mut self, event_id: &str) {
        let Some(sink) = &self.sink else { return };
        let freq: f32 = match event_id {
            "boot" => 100.0,
            "flight" => 300.0,
            "vent" => 500.0,
            _ => return,
        };
        let source = rodio::source::SineWave::new(freq)
            .take_duration(Duration::from_millis(200))
            .amplify(0.15);
        sink.mixer().add(source);
    }
}
```

  If the exact `rodio` method names above don't resolve against the
  actual compiled crate version, treat that as an expected minor
  fixup against real compiler output (same guidance as the Smash Crabs
  arc's audio task) — the *intent* (a looping quiet hum plus short
  event tones, silently no-op'd with no output device) is the
  requirement, not these exact tokens.

- [ ] **Step 3: Add the `audio` field** — change the struct definition
  (insert after `tick_count`):

```rust
    tick_count: u64,
    audio: RodioAudioSink,
    quit: bool,
}
```

  In `new()`, build the struct first, then fire the boot cue once
  `audio` exists (replace the tail of `new()`):

```rust
    fn new() -> Self {
        let mut tardis = Tardis {
            theme: tardis_theme(),
            screen: Screen::Hub,
            selected_face: 0,
            face_tween: None,
            energy: 0.0,
            vent_flash: None,
            glitch: GlitchBuffer::new(),
            particles: ParticleSystem::new(),
            transitioning_to: None,
            booting: Some(Transition::start(Duration::from_millis(BOOT_MS))),
            tick_count: 0,
            audio: RodioAudioSink::new(),
            quit: false,
        };
        tardis.audio.play("boot");
        tardis
    }
```

- [ ] **Step 4: Wire the `flight` and `vent` call sites** — in
  `update()`'s `Screen::Hub` `Enter` arm, after starting
  `transitioning_to`:

```rust
                KeyCode::Enter => {
                    if self.face_tween.is_none() {
                        if let Some(dest) = screen_for_face(self.selected_face) {
                            self.transitioning_to = Some((
                                dest,
                                Transition::start(Duration::from_millis(FLIGHT_TRANSITION_MS)),
                            ));
                            self.audio.play("flight");
                        }
                    }
                }
```

  and in `Screen::ArtronEnergy`'s `'v'` arm, after starting
  `vent_flash`:

```rust
                KeyCode::Char('v') => {
                    self.energy = (self.energy - ENERGY_VENT_AMOUNT).max(0.0);
                    self.vent_flash =
                        Some(Transition::start(Duration::from_millis(VENT_FLASH_MS)));
                    self.audio.play("vent");
                }
```

- [ ] **Step 5: Build**

Run: `cargo build --example tardis`
Expected: compiles cleanly. `rodio` and its dependency tree are already
resolved from the Smash Crabs arc's `Cargo.lock`, so this should be a
fast build, not a fresh fetch.

- [ ] **Step 6: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 7: Manual verification** (real-terminal check with audio
  hardware — cannot be performed by the implementing agent in a
  headless environment; this step requires you specifically):

Run: `cargo run --example tardis`

Confirm: a quiet continuous hum starts right at boot and keeps playing
for the whole session; a distinct tone plays at the very start of boot;
a distinct tone plays each time a flight transition starts; a distinct
tone plays on each `v` vent in Artron Energy; no audio device present
doesn't crash the app; `q` quits cleanly from every state.

- [ ] **Step 8: Commit**

```bash
git add examples/tardis.rs
git commit -m "feat(tardis): add looping hum and one-shot audio cues"
```

---

## Self-Review

**Spec coverage:** Camera/viewport/dim (magnification via `.floor()`,
out-of-bounds safety, Rgb-only dimming) — Task 1. `GlitchBuffer`
(decay via `Transition`, deterministic noise, hash-independent tests) —
Task 2. `Roundel`/`AnalogToggle`/`TimeRotor` (intensity scaling, lever
glyph swap, Braille pulse tied to speed) — Tasks 3-5. Hexagonal Hub
(6 simulated faces, hex-distance dimming, 200ms pan, instant switching
first per the ordering lesson from Smash Crabs) — Task 6. Artron
Energy (persistent `energy`, venting/lag thresholds, real `tick_rate`
slowdown, shared `TimeRotor` speed) — Task 7. Flight transition
(shake → void-streak → hard-cut arrival, 900ms, input-ignored) —
Task 8. Boot sequence (5 phases, push-through reusing the Hub's own
render, one-time only) — Task 9. Audio (looping hum via
`repeat_infinite`, 3 one-shot call sites, graceful no-device fallback)
— Task 10. Verification section (`cargo test`/`fmt`/`clippy` + full
manual `cargo run --example tardis` walkthrough covering boot through
every screen) — covered across every task's final steps. The spec's
explicit out-of-scope list (real Psychic Paper/Star Charts content,
literal rotation, new system-metrics dependency, bundled audio files) —
none added anywhere in this plan.

**Placeholder scan:** no TBD/TODO in code or commands (the `#TBD` issue
markers in task headers match the same "no filed issue number yet"
convention used in the Smash Crabs plan, not a planning gap). Task 5's
`PartialEq`-on-`Buffer` uncertainty is flagged with an explicit
fallback (per-cell comparison) rather than asserted as certain. Task
10's exact `rodio` method names are flagged as design-time-verified,
same honest caveat as the Smash Crabs arc.

**Type consistency:** `Camera::new(x, y, zoom)`, `camera::viewport`,
`camera::dim` (Task 1) match every call site in Tasks 6 and 9 exactly.
`GlitchBuffer`'s 5-method surface (Task 2) matches its only consumer,
Task 7, exactly. `Roundel::new(intensity, color)` (Task 3) is used
identically in Task 6 (ambient decoration) and Task 7 (energy pipes) —
same two-argument shape, no renames. `AnalogToggle::new(on)` (Task 4)
and `TimeRotor::new(speed)` (Task 5) match their Task 6/7/9 call sites.
`Screen`, `FACE_COUNT`, `FACE_NAMES`, `screen_for_face`, `hex_distance`,
`blit` (Task 6) are reused verbatim through Tasks 7-10 with no
signature drift. `energy`, `vent_flash`, `glitch`, `particles`
(Task 7) and `transitioning_to` (Task 8) and `booting` (Task 9) and
`audio` (Task 10) are each introduced once and read/written
consistently by every later task that touches them.
