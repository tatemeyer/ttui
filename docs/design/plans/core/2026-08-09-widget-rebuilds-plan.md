# Widget Rebuilds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild `TimeRotor`, `DamageMeter`, `EnergyCore`, and `Roundel` on the rendering primitives graduated in PR #93 (`Canvas`, `easing::lerp_color`) — real rotation instead of hash noise, continuous color gradients instead of hard thresholds, and a real filled circle for `Roundel`.

**Architecture:** All four changes are internal to their widget files (`src/widgets/{time_rotor,damage_meter,energy_core,roundel}.rs`); only `Roundel`'s constructor signature changes (`+radius: u16`), which forces call-site updates at its three TARDIS usages. No new core (`src/`, non-widget) files or APIs.

**Tech Stack:** Rust, existing `ttui` core (`canvas`, `easing`, `buffer`, `layout`).

## Global Constraints

- **Tag: `coding`. Full TDD applies to every task — no exceptions.**
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are hard gates on every task.
- **A recurring gotcha worth stating up front:** `easing::lerp_color` only interpolates when BOTH colors are `Color::Rgb` — it falls back to returning the target (`to`) color outright for any other color type (this was flagged in Arc A's final review re: `Theme.primary_end`). Every gradient in this plan (`DamageMeter`, `EnergyCore`) is written using explicit `Color::Rgb{...}` constants, not named colors like `Color::White`/`Color::Yellow`/`Color::Red` — using a named color as a lerp endpoint would silently produce a flat color instead of a gradient. Do not "simplify" any of this plan's `Color::Rgb{...}` literals back to named constants.
- One worktree for this whole Arc, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- `coding`-tagged → **Gated** autonomy tier: ships as a PR to `main` with all four required checks green, squash-merged at the end.
- Spec being implemented: `docs/design/specs/core/2026-08-09-widget-rebuilds-design.md`.

---

### Task 1: `TimeRotor` — real rotation via `Canvas`

**Files:**
- Modify: `src/widgets/time_rotor.rs`

**Interfaces:**
- Consumes: `ttui::canvas::{Canvas, CanvasMode}` (existing, unchanged).
- Produces: `TimeRotor::new(speed)`/`.render(area, tick_count, buf)` — signature unchanged, no downstream call-site changes.

- [ ] **Step 1: Write the failing tests**

Replace `src/widgets/time_rotor.rs`'s entire `#[cfg(test)] mod tests` block with:

```rust
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
    fn renders_at_least_one_braille_glyph_in_the_area() {
        let mut buf = Buffer::new(5, 4);
        TimeRotor::new(1.0).render(area(), 0, &mut buf);
        let mut found = false;
        for y in 0..4 {
            for x in 0..5 {
                if is_braille(buf.get(x, y).symbol) {
                    found = true;
                }
            }
        }
        assert!(found, "expected at least one braille glyph drawn");
    }

    #[test]
    fn identical_inputs_render_identically() {
        let mut buf_a = Buffer::new(5, 4);
        let mut buf_b = Buffer::new(5, 4);
        TimeRotor::new(2.5).render(area(), 7, &mut buf_a);
        TimeRotor::new(2.5).render(area(), 7, &mut buf_b);
        for y in 0..4 {
            for x in 0..5 {
                assert_eq!(buf_a.get(x, y), buf_b.get(x, y));
            }
        }
    }

    #[test]
    fn different_speeds_render_differently_for_the_same_tick_count() {
        let mut slow = Buffer::new(5, 4);
        let mut fast = Buffer::new(5, 4);
        TimeRotor::new(1.0).render(area(), 10, &mut slow);
        TimeRotor::new(8.0).render(area(), 10, &mut fast);
        let mut any_different = false;
        for y in 0..4 {
            for x in 0..5 {
                if slow.get(x, y) != fast.get(x, y) {
                    any_different = true;
                }
            }
        }
        assert!(
            any_different,
            "expected a visibly different rotation angle between speed 1.0 and 8.0 at tick 10"
        );
    }

    #[test]
    fn zero_size_area_does_not_panic() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        };
        TimeRotor::new(1.0).render(area, 0, &mut buf);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::time_rotor::tests`
Expected: `renders_at_least_one_braille_glyph_in_the_area` and `different_speeds_render_differently_for_the_same_tick_count` FAIL against the current hash-based implementation is not guaranteed — the current code also draws braille glyphs, just meaningless ones, so some of these tests may already pass. That's fine; the meaningful RED signal here is that these tests describe intent for the NEW implementation. Proceed to Step 3 regardless of which specific tests failed.

- [ ] **Step 3: Implement rotation via `Canvas`**

Replace the whole file's non-test content (everything above `#[cfg(test)]`) with:

```rust
//! Braille-glyph rotating speed indicator — a line sweeping through
//! the area's center at an angle driven by `tick_count * speed`.

use crate::buffer::Buffer;
use crate::canvas::{Canvas, CanvasMode};
use crate::layout::Rect;
use crossterm::style::Color;

/// Radians of rotation added per `tick_count * speed` unit — tuned so
/// the sweep is visibly moving without spinning frantically at
/// typical `speed` values (roughly 0.5-5.0).
const ROTATION_RATE: f32 = 0.05;

/// A vertical rotating-speed indicator rendered as a sweeping braille
/// line through the area's center.
pub struct TimeRotor {
    speed: f32,
}

impl TimeRotor {
    /// Creates a rotor at `speed` (floored at `0.1` so it never fully
    /// stops).
    pub fn new(speed: f32) -> Self {
        TimeRotor {
            speed: speed.max(0.1),
        }
    }

    /// Renders a line sweeping through `area`'s center, its angle
    /// driven by `tick_count` and `speed`.
    pub fn render(&self, area: Rect, tick_count: u64, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let grid_w = (area.width * 2) as f32;
        let grid_h = (area.height * 4) as f32;
        let cx = grid_w / 2.0;
        let cy = grid_h / 2.0;
        let radius = cx.min(cy).max(1.0);
        let angle = tick_count as f32 * self.speed * ROTATION_RATE;
        let (dx, dy) = (angle.cos() * radius, angle.sin() * radius);
        canvas.line(
            (cx - dx).round() as u16,
            (cy - dy).round() as u16,
            (cx + dx).round() as u16,
            (cy + dy).round() as u16,
            Color::Reset,
        );
        canvas.blit(buf, area.x, area.y);
    }
}
```

(A single line from `(cx-dx, cy-dy)` to `(cx+dx, cy+dy)` already passes through the center in both directions — no separate mirrored line call is needed. `as u16` casts on `f32` saturate to `0`/`u16::MAX` rather than panicking or wrapping, per Rust's float-to-int cast semantics since 1.45, so a negative or out-of-range coordinate degrades safely; `Canvas::set_pixel` additionally silently ignores any coordinate still out of the canvas's own bounds.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::time_rotor::tests`
Expected: PASS, all 4 tests. If `different_speeds_render_differently_for_the_same_tick_count` doesn't pass with the given `speed`/`tick_count` values (unlikely, but two angles could theoretically round to visually-identical line endpoints on a small grid), adjust the test's speed/tick constants slightly (e.g. try `speed: 12.0` instead of `8.0`) until it reliably passes — don't weaken the assertion itself.

- [ ] **Step 5: Visual check**

Run: `cargo run --example tardis` (TimeRotor is used at `examples/tardis/hub.rs:60` and `examples/tardis/artron_energy.rs:42` — signatures unchanged, no call-site edits needed). Confirm the rotor visibly sweeps/rotates rather than flickering as random noise. If the rotation looks too fast/slow, adjust `ROTATION_RATE` (this is a visual-feel constant, not a correctness requirement) and re-run.

- [ ] **Step 6: Commit**

```bash
git add src/widgets/time_rotor.rs
git commit -m "feat(widgets): rebuild TimeRotor as a real Canvas-drawn rotation

Replaces the hash-derived braille noise with an actual line sweeping
through the area's center at an angle driven by tick_count * speed —
the shape TimeRotor's name already promised."
```

---

### Task 2: `DamageMeter` — continuous color gradient

**Files:**
- Modify: `src/widgets/damage_meter.rs`

**Interfaces:**
- Consumes: `ttui::easing::lerp_color` (existing, unchanged).
- Produces: `DamageMeter::new(percent)`/`.render(area, buf)` — signature unchanged.

- [ ] **Step 1: Write the failing tests**

Replace `src/widgets/damage_meter.rs`'s entire `#[cfg(test)] mod tests` block with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::lerp_color;

    const WHITE: Color = Color::Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
    const YELLOW: Color = Color::Rgb {
        r: 255,
        g: 255,
        b: 0,
    };
    const RED: Color = Color::Rgb { r: 255, g: 0, b: 0 };

    fn area10x1() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        }
    }

    #[test]
    fn zero_percent_renders_white() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(0).render(area10x1(), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, '0');
        assert_eq!(buf.get(1, 0).symbol, '%');
        assert_eq!(buf.get(0, 0).fg, WHITE);
    }

    #[test]
    fn twenty_five_percent_is_partway_between_white_and_yellow() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(25).render(area10x1(), &mut buf);
        let fg = buf.get(0, 0).fg;
        assert_eq!(fg, lerp_color(WHITE, YELLOW, 0.5));
        assert_ne!(fg, WHITE);
        assert_ne!(fg, YELLOW);
    }

    #[test]
    fn fifty_percent_renders_exactly_yellow() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(50).render(area10x1(), &mut buf);
        assert_eq!(buf.get(0, 0).fg, YELLOW);
    }

    #[test]
    fn seventy_five_percent_is_partway_between_yellow_and_red() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(75).render(area10x1(), &mut buf);
        let fg = buf.get(0, 0).fg;
        assert_eq!(fg, lerp_color(YELLOW, RED, 0.5));
        assert_ne!(fg, YELLOW);
        assert_ne!(fg, RED);
    }

    #[test]
    fn over_100_percent_renders_red_with_full_text() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(137).render(area10x1(), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, '1');
        assert_eq!(buf.get(1, 0).symbol, '3');
        assert_eq!(buf.get(2, 0).symbol, '7');
        assert_eq!(buf.get(3, 0).symbol, '%');
        assert_eq!(buf.get(0, 0).fg, RED);
    }

    #[test]
    fn text_wider_than_area_clips_without_panic() {
        let mut buf = Buffer::new(2, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        };
        DamageMeter::new(137).render(area, &mut buf);
        assert_eq!(buf.get(0, 0).symbol, '1');
        assert_eq!(buf.get(1, 0).symbol, '3');
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::damage_meter::tests`
Expected: FAIL — `twenty_five_percent_is_partway_between_white_and_yellow` and `seventy_five_percent_is_partway_between_yellow_and_red` fail against the current 3-threshold implementation (25% currently renders solid white, not a lerp; 75% currently renders solid yellow, not a lerp). `zero_percent_renders_white`/`fifty_percent_renders_exactly_yellow`/`over_100_percent_...` may already pass by coincidence (the endpoints are the same colors) but that's fine — RED phase just needs the two gradient-specific tests to genuinely fail, which they will against named-color output vs. these `Color::Rgb` assertions.

- [ ] **Step 3: Implement the gradient**

Replace the whole file's non-test content with:

```rust
//! Percent display that continuously ramps white → yellow → red as it
//! climbs toward (and past) 100%.

use crate::buffer::{Buffer, Cell};
use crate::easing::lerp_color;
use crate::layout::Rect;
use crossterm::style::Color;

const WHITE: Color = Color::Rgb {
    r: 255,
    g: 255,
    b: 255,
};
const YELLOW: Color = Color::Rgb {
    r: 255,
    g: 255,
    b: 0,
};
const RED: Color = Color::Rgb { r: 255, g: 0, b: 0 };

/// A "N%" text readout whose color continuously ramps white → yellow
/// → red as `percent` climbs, holding solid red from 100% up.
pub struct DamageMeter {
    percent: u16,
}

impl DamageMeter {
    /// Creates a meter showing `percent` (uncapped — can exceed 100).
    pub fn new(percent: u16) -> Self {
        DamageMeter { percent }
    }

    /// Renders the percent text left-aligned in `area`.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let color = damage_color(self.percent);
        let text = format!("{}%", self.percent);
        for (i, ch) in text.chars().take(area.width as usize).enumerate() {
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol: ch,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
        }
    }
}

fn damage_color(percent: u16) -> Color {
    if percent >= 100 {
        RED
    } else if percent >= 50 {
        let t = (percent - 50) as f32 / 50.0;
        lerp_color(YELLOW, RED, t)
    } else {
        let t = percent as f32 / 50.0;
        lerp_color(WHITE, YELLOW, t)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::damage_meter::tests`
Expected: PASS, all 6 tests.

- [ ] **Step 5: Commit**

```bash
git add src/widgets/damage_meter.rs
git commit -m "feat(widgets): rebuild DamageMeter as a continuous color gradient

Replaces the two hard color-threshold jumps (white/yellow/red) with a
continuous easing::lerp_color ramp across the same three colors,
using explicit Rgb literals since lerp_color only interpolates
between Rgb colors — a named Color::White/Yellow/Red as an endpoint
would silently produce a flat color instead of a gradient."
```

---

### Task 3: `EnergyCore` — gradient fill

**Files:**
- Modify: `src/widgets/energy_core.rs`

**Interfaces:**
- Consumes: `ttui::easing::lerp_color` (existing, unchanged).
- Produces: `EnergyCore::new(percent, color)`/`.render(area, buf)` — signature unchanged.

- [ ] **Step 1: Write the failing test**

In `src/widgets/energy_core.rs`'s `#[cfg(test)] mod tests`, add `use crate::easing::lerp_color;` to the module's imports, then add:

```rust
    #[test]
    fn fill_brightens_toward_the_leading_edge() {
        let mut buf = Buffer::new(10, 1);
        let base = Color::Rgb { r: 0, g: 100, b: 0 };
        EnergyCore::new(50, base).render(area10x1(), &mut buf);
        // t=0 at the first filled column -> exactly the base color.
        assert_eq!(buf.get(0, 0).fg, base);
        // t=0.8 at the last filled column (x=4 of a 5-wide fill) ->
        // partway toward white, neither endpoint exactly.
        let leading_edge = buf.get(4, 0).fg;
        assert_eq!(
            leading_edge,
            lerp_color(base, Color::Rgb { r: 255, g: 255, b: 255 }, 0.8)
        );
        assert_ne!(leading_edge, base);
    }
```

Leave the four existing tests (`zero_percent_renders_all_empty_track`, `fifty_percent_fills_half`, `full_percent_sparks_every_fourth_cell`, `zero_width_area_does_not_panic`) unchanged — none of them assert `fg` on a filled non-spark cell, so all four remain valid under the gradient change without modification.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib widgets::energy_core::tests`
Expected: FAIL — `fill_brightens_toward_the_leading_edge`'s two `fg` assertions fail against the current flat-`self.color` implementation (every filled cell currently renders exactly `base`, not a gradient).

- [ ] **Step 3: Implement the gradient fill**

Change:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

/// A horizontal segmented progress bar filled to `percent` in
/// `color`.
pub struct EnergyCore {
    percent: u16,
    color: Color,
}
```

to:

```rust
use crate::buffer::{Buffer, Cell};
use crate::easing::lerp_color;
use crate::layout::Rect;
use crossterm::style::Color;

const WHITE: Color = Color::Rgb {
    r: 255,
    g: 255,
    b: 255,
};

/// A horizontal segmented progress bar filled to `percent` in
/// `color`, brightening toward white across the fill. `color` should
/// be `Color::Rgb` for the gradient to interpolate — `easing::
/// lerp_color`'s existing fallback renders every filled cell flat
/// white for any other color type.
pub struct EnergyCore {
    percent: u16,
    color: Color,
}
```

and change the fill branch inside `render`:

```rust
            } else if filled {
                ('▓', self.color)
```

to:

```rust
            } else if filled {
                let t = x as f32 / filled_width.max(1) as f32;
                ('▓', lerp_color(self.color, WHITE, t))
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::energy_core::tests`
Expected: PASS, all 5 tests.

- [ ] **Step 5: Commit**

```bash
git add src/widgets/energy_core.rs
git commit -m "feat(widgets): rebuild EnergyCore fill as a brightening gradient

Filled cells now lerp from the base color toward white across the
fill instead of rendering flat — a 'charging up' look. color should
be Color::Rgb for this to interpolate, per lerp_color's existing
non-Rgb fallback (documented on the struct)."
```

---

### Task 4: `Roundel` — real filled circle + `radius` parameter

**Files:**
- Modify: `src/widgets/roundel.rs`

**Interfaces:**
- Consumes: `ttui::canvas::{Canvas, CanvasMode}` (existing, unchanged).
- Produces: `Roundel::new(intensity, color, radius: u16)` — **signature change** (was `new(intensity, color)`), used by Task 5.

- [ ] **Step 1: Write the failing tests**

Replace `src/widgets/roundel.rs`'s entire `#[cfg(test)] mod tests` block with:

```rust
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
    fn radius_zero_zero_intensity_renders_near_black() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            0,
        )
        .render(area3x3(), &mut buf);

        assert_eq!(buf.get(1, 1).fg, Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn radius_zero_full_intensity_renders_the_input_color_unchanged() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            1.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            0,
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
    fn radius_zero_half_intensity_halves_each_channel() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.5,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            0,
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
    fn radius_zero_renders_at_area_center_and_does_not_panic_on_a_1x1_area() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };

        Roundel::new(1.0, Color::White, 0).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'O');
    }

    #[test]
    fn radius_one_renders_a_circle_spanning_multiple_cells() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            1.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            1,
        )
        .render(area3x3(), &mut buf);

        let mut non_default = 0;
        for y in 0..3 {
            for x in 0..3 {
                if *buf.get(x, y) != Cell::default() {
                    non_default += 1;
                }
            }
        }
        assert!(
            non_default > 1,
            "expected a circle spanning multiple cells, got {non_default}"
        );
    }

    #[test]
    fn radius_one_zero_intensity_still_renders_near_black() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            1,
        )
        .render(area3x3(), &mut buf);

        assert_eq!(buf.get(1, 1).fg, Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn radius_one_on_a_tiny_area_does_not_panic() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        Roundel::new(1.0, Color::White, 1).render(area, &mut buf);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::roundel::tests`
Expected: FAIL to compile — `Roundel::new` doesn't accept a third argument yet.

- [ ] **Step 3: Implement the `radius` parameter and circle rendering**

Replace the whole file's non-test content with:

```rust
//! Pulsing circular decoration glyph — a single-cell glyph
//! (`radius: 0`) or a real filled circle (`radius >= 1`) whose
//! brightness the owning app drives per frame via `intensity`.

use crate::buffer::{Buffer, Cell};
use crate::canvas::{Canvas, CanvasMode};
use crate::layout::Rect;
use crossterm::style::Color;

/// A glyph, or a filled circle at `radius >= 1`, whose
/// brightness/fill reflects `intensity`.
pub struct Roundel {
    intensity: f32,
    color: Color,
    radius: u16,
}

impl Roundel {
    /// Creates a roundel at `intensity` (clamped to `0.0..=1.0`) in
    /// `color`. `radius: 0` renders the original single-glyph `'O'`
    /// at `area`'s center (unchanged from before this widget grew a
    /// circle mode); `radius >= 1` renders a filled circle roughly
    /// `radius * 2 + 1` cells across, via `Canvas`.
    pub fn new(intensity: f32, color: Color, radius: u16) -> Self {
        Roundel {
            intensity: intensity.clamp(0.0, 1.0),
            color,
            radius,
        }
    }

    /// Renders at `area`'s center: a single glyph at `radius: 0`, or
    /// a filled circle at `radius >= 1`.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let scaled = scale_color(self.color, self.intensity);
        if self.radius == 0 {
            let cx = area.x + area.width / 2;
            let cy = area.y + area.height / 2;
            buf.set(
                cx,
                cy,
                Cell {
                    symbol: 'O',
                    fg: scaled,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            return;
        }
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let grid_w = area.width * 2;
        let grid_h = area.height * 4;
        let cx = grid_w as f32 / 2.0;
        let cy = grid_h as f32 / 2.0;
        // A typical monospace cell is roughly twice as tall as wide
        // in real pixels, and braille's 2-wide x 4-tall dot grid
        // divides each cell in exactly that ratio — so each dot is
        // roughly square in real screen space, and a plain Euclidean
        // distance in dot-coordinate space already reads as round
        // without any aspect-ratio correction.
        let subpixel_radius = self.radius as f32 * 2.0 + 1.0;
        for y in 0..grid_h {
            for x in 0..grid_w {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if (dx * dx + dy * dy).sqrt() <= subpixel_radius {
                    canvas.set_pixel(x, y, scaled);
                }
            }
        }
        canvas.blit(buf, area.x, area.y);
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
Expected: PASS, all 7 tests.

- [ ] **Step 5: Commit**

```bash
git add src/widgets/roundel.rs
git commit -m "feat(widgets): give Roundel a radius parameter and a real circle mode

radius: 0 keeps the original single-glyph rendering unchanged (needed
by star_charts.rs's tightly-spaced timeline rows); radius >= 1 draws
a filled circle via Canvas, for the two ambient-pulse call sites that
have room for it. Downstream call-site updates land in the next task."
```

---

### Task 5: TARDIS call-site updates for `Roundel`'s new signature

**Files:**
- Modify: `examples/tardis/artron_energy.rs`
- Modify: `examples/tardis/hub.rs`
- Modify: `examples/tardis/star_charts.rs`

**Interfaces:**
- Consumes: `Roundel::new(intensity, color, radius)` (Task 4).
- Produces: a compiling workspace (this is a pure mechanical fixup — no new test-first cycle applies, same reasoning already established in the rendering-primitives-graduation plan for pure call-site migrations; the workspace build is the regression check).

- [ ] **Step 1: `examples/tardis/artron_energy.rs`**

Change:

```rust
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
```

to:

```rust
        for i in 0..3u16 {
            let seg_intensity = ((self.energy - i as f32 * 33.0) / 33.0).clamp(0.0, 1.0);
            let rx = area.x + 4 + i * 4;
            let ry = area.y + 2;
            Roundel::new(seg_intensity, self.theme.tertiary, 1).render(
                Rect {
                    x: rx.saturating_sub(1),
                    y: ry.saturating_sub(1),
                    width: 3,
                    height: 3,
                },
                buf,
            );
        }
```

(`rx`/`ry` previously marked the single cell the roundel occupied; a 3-wide/3-tall area centered on that same point starts 1 cell up-and-left of it.)

- [ ] **Step 2: `examples/tardis/hub.rs`**

Change:

```rust
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
```

to:

```rust
            for i in 0..3u16 {
                let rx = area.x + (area.width / 4) * (i + 1);
                let ry = area.y + area.height / 2;
                let pulse = ((self.tick_count as f32 * 0.05 + i as f32).sin() + 1.0) / 2.0;
                Roundel::new(pulse, self.theme.tertiary, 1).render(
                    Rect {
                        x: rx.saturating_sub(1),
                        y: ry.saturating_sub(1),
                        width: 3,
                        height: 3,
                    },
                    buf,
                );
            }
```

- [ ] **Step 3: `examples/tardis/star_charts.rs`**

Change:

```rust
                Roundel::new(pulse, self.theme.primary).render(
                    Rect {
                        x: area.x,
                        y: area.y + row,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
```

to:

```rust
                Roundel::new(pulse, self.theme.primary, 0).render(
                    Rect {
                        x: area.x,
                        y: area.y + row,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
```

(Only the constructor call gains `, 0` — the `Rect` is intentionally unchanged, since this call site stays on the compact single-glyph path to avoid overlapping adjacent timeline rows, which are only 1 cell apart.)

- [ ] **Step 4: Build and test the whole workspace**

Run: `cargo build --all-targets`
Expected: succeeds.

Run: `cargo test`
Expected: full suite passes.

- [ ] **Step 5: Commit**

```bash
git add examples/tardis/artron_energy.rs examples/tardis/hub.rs examples/tardis/star_charts.rs
git commit -m "fix(tardis): update Roundel call sites for its new radius parameter

artron_energy and hub switch to radius 1 (a real circle, their 4-cell
spacing comfortably fits it); star_charts stays at radius 0 (compact
single glyph) since its timeline rows are only 1 cell apart."
```

---

### Task 6: Final workspace verification

**Files:** none (verification only).

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: full suite green, including every test added/changed across Tasks 1-5.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds.

- [ ] **Step 4: Manual visual regression check**

Run `cargo run --example tardis`: confirm the `artron_energy`/`hub` roundels render as visible circles with no overlap; the `star_charts` roundel is unchanged (a single pulsing glyph); the time rotor visibly sweeps/rotates.

Run `cargo run --example smash_crabs`: confirm `DamageMeter` (used by the Versus Mode sub-app) shows a smooth color ramp rather than two hard jumps.

Run `cargo run --example omnitrix`: confirm `EnergyCore` (used by the Fasttrack sub-app) shows a visible brightness gradient across its fill rather than a flat bar.

Press `q` to quit each.

- [ ] **Step 5: Commit (if Step 4 required any fix) or proceed**

If Step 4 surfaces no issues, there is nothing to commit for this task.

---

## Final verification (whole plan)

- [ ] `cargo test` — full suite green.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo build --all-targets` — library, examples, benches all compile.
- [ ] Manual visual check on TARDIS/Smash Crabs/Omnitrix confirms all four rebuilt widgets render correctly with no regression in the surrounding UI.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree`.
