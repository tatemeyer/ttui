# Mission Control — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-12
**Relationship to prior specs:** the last of the four future Arcs
named in the original post-windshield brainstorm (rendering depth/
perspective and advanced input handling are both done and merged).
Introduces a new themed example app — following the same genesis
pattern as Falcon/Omnitrix/Tardis/Smash Crabs, each of which paired a
new example with the `src/widgets/` primitives it needed. No code
dependency on any prior Arc beyond the existing `App` trait
(`src/app.rs`), `Layout`/`Constraint` (`src/layout.rs`), and the
existing `Block` widget (`src/widgets/block.rs`).

## Problem

This project has no widgets for displaying numeric data — the closest
existing pieces (`DamageMeter`, `Dial`, `EnergyCore`) are single-value
gauges, not built for showing a value *compared against others* or *a
trend over time*. This spec adds two general-purpose primitives (a bar
chart and a sparkline) and proves them with a new example — a ground-
control telemetry console — rather than shipping them untested against
a real render loop.

## Scope

**Tag: `coding`, TDD mandatory** for the two new `src/widgets/`
primitives — no exemption, matching every other `src/` widget's
existing test coverage (see `damage_meter.rs`'s test module for the
established style: exact-value assertions on rendered cells, plus a
no-panic case for a too-small area).

**Tag: `coding`, TDD-exempt** for `examples/mission_control.rs` per the
"Examples/demos" exception in `.claude/rules/development-conventions.md`
— correctness verified by running (`tools/visual-snapshot`, mandatory
since this is a new example's `view()`/`on_tick()`).

Three slices, in dependency order:

1. **`BarChart` widget** (`src/widgets/bar_chart.rs`)
2. **`Sparkline` widget** (`src/widgets/sparkline.rs`) — independent of 1.
3. **Mission Control example** (`examples/mission_control.rs`) — depends
   on 1-2.

## Design

### Slice 1: `BarChart`

```rust
//! Horizontal bar chart: one labeled row per item, bar length scaled
//! to a shared maximum, all bars left-aligned at a common column.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

/// A horizontal bar chart — one row per `(label, value)` pair, bar
/// length scaled against `max`.
pub struct BarChart<'a> {
    items: &'a [(&'a str, f32)],
    max: f32,
    color: Color,
}

impl<'a> BarChart<'a> {
    /// Creates a chart over `items`, with bar lengths scaled against
    /// `max` (a value exceeding `max` draws a full-width bar — not
    /// clamped as an error, just visually capped).
    pub fn new(items: &'a [(&'a str, f32)], max: f32, color: Color) -> Self {
        BarChart { items, max, color }
    }

    /// Renders one row per item (truncated to `area.height` rows).
    /// All labels are truncated/padded to the longest label's width
    /// so every bar starts at the same column, then filled with `█`
    /// cells proportional to `value / max`.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let label_width = self
            .items
            .iter()
            .map(|(l, _)| l.chars().count())
            .max()
            .unwrap_or(0) as u16;
        for (row, (label, value)) in self.items.iter().take(area.height as usize).enumerate() {
            let y = area.y + row as u16;
            let mut x = area.x;
            for (i, ch) in label.chars().enumerate() {
                if i as u16 >= label_width || x >= area.x + area.width {
                    break;
                }
                buf.set(
                    x,
                    y,
                    Cell {
                        symbol: ch,
                        fg: self.color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                x += 1;
            }
            x = (area.x + label_width + 1).min(area.x + area.width);
            let bar_space = (area.x + area.width).saturating_sub(x);
            let fraction = if self.max > 0.0 {
                (value / self.max).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let filled = (fraction * bar_space as f32).round() as u16;
            for i in 0..filled.min(bar_space) {
                buf.set(
                    x + i,
                    y,
                    Cell {
                        symbol: '█',
                        fg: self.color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
        }
    }
}
```

No sub-cell (fractional-block) precision in v1 — whole `█` cells only,
matching `DamageMeter`'s existing simplicity over reaching for
`Canvas`'s Braille sub-pixel machinery.

### Slice 2: `Sparkline`

```rust
//! Compact single-line trend indicator: one column per value, no
//! axes, auto-scaled to its own data.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A single-row sparkline — one column per value (a trailing window
/// if there are more values than `area.width`), auto-scaled to the
/// slice's own min/max.
pub struct Sparkline<'a> {
    values: &'a [f32],
    color: Color,
}

impl<'a> Sparkline<'a> {
    /// Creates a sparkline over `values`.
    pub fn new(values: &'a [f32], color: Color) -> Self {
        Sparkline { values, color }
    }

    /// Renders the trailing `area.width` values as one row of
    /// height-coded block glyphs at `area.y` — always exactly one
    /// row, regardless of `area.height` (matching a sparkline's whole
    /// purpose: a compact single line, not a filled chart).
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.values.is_empty() {
            return;
        }
        let window_len = (area.width as usize).min(self.values.len());
        let window = &self.values[self.values.len() - window_len..];
        let min = window.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = window.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        for (i, &value) in window.iter().enumerate() {
            let level = if max > min {
                (((value - min) / (max - min)) * (LEVELS.len() - 1) as f32).round() as usize
            } else {
                LEVELS.len() / 2
            };
            let symbol = LEVELS[level.min(LEVELS.len() - 1)];
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol,
                    fg: self.color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
        }
    }
}
```

`max > min` guards the flat-data case (all values identical, or a
single value) — falls back to the middle level rather than dividing by
zero. Newest data is always at the rightmost column (a trailing
window drops the *oldest* values when there are more than fit, not the
newest), matching how every real sparkline reads chronologically.

Both widgets registered in `src/widgets/mod.rs` (alphabetical, one-line
`///` each) — 16 widgets total, still within the 15-20-per-directory
soft ceiling in `.claude/rules/development-conventions.md`, so no
subdirectory reorganization is in scope for this Arc.

### Slice 3: Mission Control example

A new flat single-file example (`examples/mission_control.rs`, no
multi-file `mod` structure — matches `demo.rs`/`depth_spike.rs`'s
pattern, not the more complex `falcon/` directory, since nothing here
needs a boot sequence or per-screen split). A 2×2 grid of `Block`-
bordered telemetry panels, three sparklines and one bar chart, all
animating via the same deterministic `scatter()`-hash pseudo-random
walk pattern every prior Arc has used (no RNG dependency):

```rust
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::LayerStack;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{bar_chart::BarChart, block::Block, sparkline::Sparkline};

const TICK_INTERVAL: Duration = Duration::from_millis(33);
const HISTORY_LEN: usize = 40;
const ALTITUDE_STEP: f32 = 15.0;
const VELOCITY_STEP: f32 = 8.0;
const SIGNAL_STEP: f32 = 4.0;
const SUBSYSTEM_STEP: f32 = 3.0;

const SUBSYSTEM_NAMES: [&str; 5] = ["Engines", "Life Support", "Comms", "Nav", "Power"];

fn mission_control_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 5, g: 10, b: 15 },
        primary: Color::Rgb { r: 80, g: 200, b: 255 },
        secondary: Color::Rgb { r: 230, g: 230, b: 230 },
        tertiary: Color::Rgb { r: 255, g: 60, b: 60 },
        accent: Color::Rgb { r: 255, g: 180, b: 60 },
        primary_end: None,
        border: BorderSet::default(),
        border_bold: false,
        border_thick: false,
    }
}

fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}

struct MissionControl {
    theme: Theme,
    altitude: f32,
    altitude_history: Vec<f32>,
    velocity: f32,
    velocity_history: Vec<f32>,
    signal: f32,
    signal_history: Vec<f32>,
    subsystems: [f32; 5],
    tick_count: u64,
    quit: bool,
}

impl MissionControl {
    fn new() -> Self {
        MissionControl {
            theme: mission_control_theme(),
            altitude: 5000.0,
            altitude_history: vec![5000.0],
            velocity: 250.0,
            velocity_history: vec![250.0],
            signal: 80.0,
            signal_history: vec![80.0],
            subsystems: [95.0, 92.0, 88.0, 90.0, 97.0],
            tick_count: 0,
            quit: false,
        }
    }

    fn push_history(history: &mut Vec<f32>, value: f32) {
        history.push(value);
        if history.len() > HISTORY_LEN {
            history.remove(0);
        }
    }
}

impl App for MissionControl {
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
        let rows = Layout::new(Direction::Vertical, vec![Constraint::Fill(1); 2]).split(area);
        let top = Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 2]).split(rows[0]);
        let bottom = Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 2]).split(rows[1]);

        let altitude_inner = Block::new()
            .title("Altitude (m)")
            .theme(&self.theme)
            .render(top[0], buf);
        Sparkline::new(&self.altitude_history, self.theme.primary).render(altitude_inner, buf);

        let velocity_inner = Block::new()
            .title("Velocity (m/s)")
            .theme(&self.theme)
            .render(top[1], buf);
        Sparkline::new(&self.velocity_history, self.theme.primary).render(velocity_inner, buf);

        let signal_inner = Block::new()
            .title("Signal Strength (%)")
            .theme(&self.theme)
            .render(bottom[0], buf);
        Sparkline::new(&self.signal_history, self.theme.accent).render(signal_inner, buf);

        let subsystem_items: Vec<(&str, f32)> = SUBSYSTEM_NAMES
            .iter()
            .zip(self.subsystems.iter())
            .map(|(&name, &health)| (name, health))
            .collect();
        let subsystem_inner = Block::new()
            .title("Subsystem Status")
            .theme(&self.theme)
            .render(bottom[1], buf);
        BarChart::new(&subsystem_items, 100.0, self.theme.secondary).render(subsystem_inner, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, _elapsed: Duration) {
        self.tick_count += 1;
        let base = self.tick_count as u32;
        self.altitude = (self.altitude + scatter(base, ALTITUDE_STEP)).clamp(0.0, 10_000.0);
        Self::push_history(&mut self.altitude_history, self.altitude);
        self.velocity =
            (self.velocity + scatter(base.wrapping_add(1_000), VELOCITY_STEP)).clamp(0.0, 500.0);
        Self::push_history(&mut self.velocity_history, self.velocity);
        self.signal =
            (self.signal + scatter(base.wrapping_add(2_000), SIGNAL_STEP)).clamp(0.0, 100.0);
        Self::push_history(&mut self.signal_history, self.signal);
        for (i, health) in self.subsystems.iter_mut().enumerate() {
            *health = (*health
                + scatter(base.wrapping_add(3_000 + i as u32 * 777), SUBSYSTEM_STEP))
            .clamp(0.0, 100.0);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut app = MissionControl::new();
    run(&mut app)
}
```

Deliberately no Tab-cycling/focus mechanic and no other input beyond
`q` to quit — all four panels animate simultaneously and equally, since
the subject of this Arc is the two new widgets, not another custom
input scheme (the project already has a general one, from the prior
Arc). `examples/README.md` gains a one-sentence entry for this app, per
`.claude/rules/development-conventions.md`'s docs-organization rule.

## Non-goals

- **Sub-cell (fractional-block) bar precision.** `BarChart` bars are
  whole `█` cells only.
- **Labeled/scaled axes, legends, or multi-series overlay charts.**
  Explicitly the "something broader" option not chosen during
  brainstorming — both widgets are intentionally minimal.
- **Any interaction beyond `q` to quit** in Mission Control — no
  focus/selection mechanic.
- **Any `src/widgets/` directory reorganization.** 16 files is still
  within the soft ceiling; revisit only if a future Arc pushes further.
- **Any change to Falcon, or any other existing example.** This Arc
  is entirely additive: two new widgets, one new example.

## Testing

Per `.claude/rules/development-conventions.md`: `src/widgets/bar_chart.rs`
and `src/widgets/sparkline.rs` are `coding`-tagged with full TDD, no
exception. Concrete behaviors the test suites must cover:

**`BarChart`:**
- A single item's bar length is proportional to `value / max`.
- A value equal to `max` fills the full available bar width.
- A value exceeding `max` still renders a full-width bar (capped, not
  overflowing or panicking).
- Multiple items with different label lengths all have bars starting
  at the same column (the longest label's width + 1).
- More items than `area.height` truncates to the available rows
  without panicking.
- Zero-width or zero-height area renders nothing, no panic.
- `max <= 0.0` renders empty bars (no divide-by-zero panic).

**`Sparkline`:**
- Values render at the correct relative height (a higher value gets a
  taller glyph than a lower one in the same window).
- Flat data (all equal values) renders the middle-level glyph for
  every column, not a panic from a zero-width min/max range.
- More values than `area.width` shows only the trailing window (the
  most recent values), dropping the oldest.
- Fewer values than `area.width` renders only that many columns, no
  padding/panic.
- Empty `values` slice renders nothing, no panic.
- Renders on exactly one row regardless of `area.height`.

Mission Control (TDD-exempt, `tools/visual-snapshot` mandatory):
capture the full 2×2 grid post-startup, confirming three visibly
distinct sparklines and one bar chart with five labeled, differently-
sized bars, plus a second capture a few hundred ms later confirming
all four panels have visibly changed (proving the tick-driven
animation is live).

## Critical files

- `src/widgets/bar_chart.rs`, `src/widgets/sparkline.rs` — new widgets.
- `src/widgets/mod.rs` — two new `pub mod` entries.
- `examples/mission_control.rs` — new example.
- `examples/README.md` — new entry.

## Verification

- `cargo build --all-targets` / `cargo clippy --all-targets -- -D
  warnings` / `cargo fmt --check` — clean.
- `cargo test` — all new `bar_chart`/`sparkline` unit tests green, full
  existing suite unchanged.
- `tools/visual-snapshot` capture of Mission Control per the Testing
  section above, `Read` and confirmed, not just claimed.
