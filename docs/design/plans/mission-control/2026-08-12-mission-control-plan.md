# Mission Control Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two general-purpose data-viz widgets (`BarChart`, `Sparkline`) to `src/widgets/`, and prove them with a new example — a ground-control telemetry console.

**Architecture:** Both widgets follow the existing "dumb" widget convention (`render(&self, area: Rect, buf: &mut Buffer)`, no internal state) established by `DamageMeter`/`Block`/etc. The new example composes them with the existing `Block` and `Layout` primitives into a 2x2 grid, animated via the same deterministic `scatter()`-hash pattern every prior Arc has used.

**Tech Stack:** Rust, `crossterm::style::Color`, existing `ttui::buffer`/`ttui::layout` primitives.

## Global Constraints

- **`src/widgets/bar_chart.rs` and `src/widgets/sparkline.rs` are `coding`-tagged with full TDD mandatory** — no exemption.
- **`examples/mission_control.rs` is TDD-exempt** (example code, "Examples/demos" exception in `.claude/rules/development-conventions.md`), but **`tools/visual-snapshot` is mandatory** for that task.
- **`BarChart` bars are whole `█` cells only** — no sub-cell/fractional-block precision.
- **`Sparkline` always renders on exactly one row**, regardless of `area.height`, and always shows the *trailing* window (most recent values) when there are more values than `area.width`.
- **No axes, legends, or multi-series charts. No `src/widgets/` directory reorganization** (16 files stays within the existing 15-20-per-directory soft ceiling). **No changes to any existing example.**

---

### Task 1: `BarChart` widget

**Files:**
- Create: `src/widgets/bar_chart.rs`
- Modify: `src/widgets/mod.rs` (add `pub mod bar_chart;`, alphabetically — between `analog_toggle` and `block`)

**Interfaces:**
- Produces: `pub struct BarChart<'a>`, `impl<'a> BarChart<'a> { pub fn new(items: &'a [(&'a str, f32)], max: f32, color: Color) -> Self; pub fn render(&self, area: Rect, buf: &mut Buffer); }` — consumed by Task 3.

- [ ] **Step 1: Write the failing tests**

Create `src/widgets/bar_chart.rs` with:

```rust
//! Horizontal bar chart: one labeled row per item, bar length scaled
//! to a shared maximum, all bars left-aligned at a common column.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Buffer, Cell};

    fn area(width: u16, height: u16) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    #[test]
    fn bar_length_is_proportional_to_value_over_max() {
        let items = [("A", 50.0)];
        let mut buf = Buffer::new(20, 1);
        BarChart::new(&items, 100.0, Color::Reset).render(area(20, 1), &mut buf);
        // label "A" (1 char) + 1 space -> bar starts at x=2, bar_space=18
        // fraction 0.5 -> filled = round(0.5*18) = 9
        for i in 0..9 {
            assert_eq!(buf.get(2 + i, 0).symbol, '█', "expected filled at offset {i}");
        }
        assert_ne!(buf.get(2 + 9, 0).symbol, '█');
    }

    #[test]
    fn value_equal_to_max_fills_full_bar_width() {
        let items = [("X", 100.0)];
        let mut buf = Buffer::new(10, 1);
        BarChart::new(&items, 100.0, Color::Reset).render(area(10, 1), &mut buf);
        // label "X" (1) + 1 space -> bar starts at x=2, bar_space=8
        for i in 0..8 {
            assert_eq!(buf.get(2 + i, 0).symbol, '█');
        }
    }

    #[test]
    fn value_exceeding_max_still_fills_only_the_full_bar_width() {
        let items = [("X", 999.0)];
        let mut buf = Buffer::new(10, 1);
        BarChart::new(&items, 100.0, Color::Reset).render(area(10, 1), &mut buf);
        for i in 0..8 {
            assert_eq!(buf.get(2 + i, 0).symbol, '█');
        }
    }

    #[test]
    fn bars_align_to_the_longest_label_across_items() {
        let items = [("A", 50.0), ("Longer", 50.0)];
        let mut buf = Buffer::new(30, 2);
        BarChart::new(&items, 100.0, Color::Reset).render(area(30, 2), &mut buf);
        // "Longer" is 6 chars -> label_width=6 -> both bars start at x=7
        assert_eq!(buf.get(0, 0).symbol, 'A');
        assert_eq!(buf.get(7, 0).symbol, '█');
        assert_eq!(buf.get(7, 1).symbol, '█');
    }

    #[test]
    fn more_items_than_area_height_truncates_without_panic() {
        let items = [("A", 10.0), ("B", 10.0), ("C", 10.0)];
        let mut buf = Buffer::new(10, 2);
        BarChart::new(&items, 100.0, Color::Reset).render(area(10, 2), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, 'A');
        assert_eq!(buf.get(0, 1).symbol, 'B');
    }

    #[test]
    fn zero_width_or_zero_height_area_renders_nothing_without_panic() {
        let items = [("A", 50.0)];
        let mut buf = Buffer::new(10, 10);
        BarChart::new(&items, 100.0, Color::Reset).render(
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 5,
            },
            &mut buf,
        );
        BarChart::new(&items, 100.0, Color::Reset).render(
            Rect {
                x: 0,
                y: 0,
                width: 5,
                height: 0,
            },
            &mut buf,
        );
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn max_of_zero_renders_empty_bars_without_panic() {
        let items = [("A", 50.0)];
        let mut buf = Buffer::new(10, 1);
        BarChart::new(&items, 0.0, Color::Reset).render(area(10, 1), &mut buf);
        for x in 2..10 {
            assert_ne!(buf.get(x, 0).symbol, '█');
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib bar_chart::`
Expected: FAIL to compile — `BarChart`/`Rect`/`Color` aren't imported/don't exist yet in this new file.

- [ ] **Step 3: Write the implementation**

Add this above the `#[cfg(test)]` block in `src/widgets/bar_chart.rs`:

```rust
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

In `src/widgets/mod.rs`, add (alphabetically, between `analog_toggle` and `block`):

```rust
/// Horizontal bar chart with labeled, max-scaled bars.
pub mod bar_chart;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib bar_chart::`
Expected: all 7 tests PASS.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 6: Commit**

```bash
git add src/widgets/bar_chart.rs src/widgets/mod.rs
git commit -m "feat(widgets): add BarChart

Horizontal bar chart, one row per labeled item, bars scaled to a
shared maximum and aligned to the longest label's width."
```

---

### Task 2: `Sparkline` widget

**Files:**
- Create: `src/widgets/sparkline.rs`
- Modify: `src/widgets/mod.rs` (add `pub mod sparkline;`, alphabetically — between `smash_border` and `table`)

**Interfaces:**
- Produces: `pub struct Sparkline<'a>`, `impl<'a> Sparkline<'a> { pub fn new(values: &'a [f32], color: Color) -> Self; pub fn render(&self, area: Rect, buf: &mut Buffer); }` — consumed by Task 3. Independent of Task 1.

- [ ] **Step 1: Write the failing tests**

Create `src/widgets/sparkline.rs` with:

```rust
//! Compact single-line trend indicator: one column per value, no
//! axes, auto-scaled to its own data.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Buffer, Cell};

    fn area(width: u16, height: u16) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    #[test]
    fn higher_value_renders_a_taller_glyph_than_a_lower_one() {
        let values = [0.0, 100.0];
        let mut buf = Buffer::new(2, 1);
        Sparkline::new(&values, Color::Reset).render(area(2, 1), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, LEVELS[0]);
        assert_eq!(buf.get(1, 0).symbol, LEVELS[7]);
    }

    #[test]
    fn flat_data_renders_the_middle_level_glyph_without_panic() {
        let values = [42.0, 42.0, 42.0];
        let mut buf = Buffer::new(3, 1);
        Sparkline::new(&values, Color::Reset).render(area(3, 1), &mut buf);
        for x in 0..3 {
            assert_eq!(buf.get(x, 0).symbol, LEVELS[4]);
        }
    }

    #[test]
    fn more_values_than_area_width_shows_only_the_trailing_window() {
        let values = [0.0, 100.0, 50.0];
        let mut buf = Buffer::new(2, 1);
        Sparkline::new(&values, Color::Reset).render(area(2, 1), &mut buf);
        // trailing window = last 2 values [100.0, 50.0]; min=50, max=100
        assert_eq!(buf.get(0, 0).symbol, LEVELS[7]);
        assert_eq!(buf.get(1, 0).symbol, LEVELS[0]);
    }

    #[test]
    fn fewer_values_than_area_width_renders_only_that_many_columns() {
        let values = [10.0, 20.0];
        let mut buf = Buffer::new(5, 1);
        Sparkline::new(&values, Color::Reset).render(area(5, 1), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, LEVELS[0]);
        assert_eq!(buf.get(1, 0).symbol, LEVELS[7]);
        assert_eq!(buf.get(2, 0).symbol, ' ');
    }

    #[test]
    fn empty_values_renders_nothing_without_panic() {
        let values: [f32; 0] = [];
        let mut buf = Buffer::new(5, 1);
        Sparkline::new(&values, Color::Reset).render(area(5, 1), &mut buf);
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn renders_on_exactly_one_row_regardless_of_area_height() {
        let values = [10.0, 20.0];
        let mut buf = Buffer::new(5, 3);
        Sparkline::new(&values, Color::Reset).render(area(5, 3), &mut buf);
        assert_ne!(buf.get(0, 0).symbol, ' ');
        assert_eq!(buf.get(0, 1).symbol, ' ');
        assert_eq!(buf.get(0, 2).symbol, ' ');
    }

    #[test]
    fn zero_width_or_zero_height_area_renders_nothing_without_panic() {
        let values = [10.0, 20.0];
        let mut buf = Buffer::new(5, 5);
        Sparkline::new(&values, Color::Reset).render(
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 3,
            },
            &mut buf,
        );
        Sparkline::new(&values, Color::Reset).render(
            Rect {
                x: 0,
                y: 0,
                width: 3,
                height: 0,
            },
            &mut buf,
        );
        assert_eq!(*buf.get(0, 0), Cell::default());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib sparkline::`
Expected: FAIL to compile — `Sparkline`/`LEVELS` don't exist yet.

- [ ] **Step 3: Write the implementation**

Add this above the `#[cfg(test)]` block in `src/widgets/sparkline.rs`:

```rust
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
    /// row, regardless of `area.height`.
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

In `src/widgets/mod.rs`, add (alphabetically, between `smash_border` and `table`):

```rust
/// Single-row auto-scaled trend indicator.
pub mod sparkline;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib sparkline::`
Expected: all 7 tests PASS.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.
Run: `cargo test` — full suite green (both `bar_chart::` and `sparkline::` tests included).

- [ ] **Step 6: Commit**

```bash
git add src/widgets/sparkline.rs src/widgets/mod.rs
git commit -m "feat(widgets): add Sparkline

Single-row, auto-scaled trend indicator using the standard 8-level
Unicode block glyph set, showing a trailing window when there are
more values than the render area is wide."
```

---

### Task 3: Mission Control example

**Files:**
- Create: `examples/mission_control.rs`
- Modify: `examples/README.md` (new entry)

**Interfaces:**
- Consumes: `ttui::widgets::{bar_chart::BarChart, sparkline::Sparkline, block::Block}` (Tasks 1-2 + existing), `ttui::app::{run, App}`, `ttui::layout::{Constraint, Direction, Layout, Rect}`, `ttui::buffer::LayerStack`, `ttui::theme::{BorderSet, Theme}` (all pre-existing).
- Produces: nothing consumed by later tasks (Task 4 is verification-only).

- [ ] **Step 1: Write the example**

Create `examples/mission_control.rs`:

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
        primary: Color::Rgb {
            r: 80,
            g: 200,
            b: 255,
        },
        secondary: Color::Rgb {
            r: 230,
            g: 230,
            b: 230,
        },
        tertiary: Color::Rgb { r: 255, g: 60, b: 60 },
        accent: Color::Rgb {
            r: 255,
            g: 180,
            b: 60,
        },
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
        let bottom =
            Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 2]).split(rows[1]);

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

- [ ] **Step 2: Add the `examples/README.md` entry**

Open `examples/README.md` and add one entry for Mission Control, matching the existing entries' style/length (name, one-sentence description, vision doc pointer):

```markdown
- **`mission_control`** — a NASA-style ground-control telemetry console:
  three live sparklines (altitude, velocity, signal strength) and a
  bar chart of subsystem health, all animating via a deterministic
  random walk. Built from
  `docs/design/specs/mission-control/2026-08-12-mission-control-design.md`.
```

- [ ] **Step 3: Build, lint, format**

Run: `cargo build --example mission_control` — succeeds, no warnings.
Run: `cargo clippy --example mission_control -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 4: Capture and verify visually**

Using `tools/visual-snapshot`, capture two frames a few hundred ms apart post-startup (e.g. a script of `[{"wait_ms": 100}, {"wait_ms": 400}]`):
```
cargo run -p visual-snapshot -- --example mission_control --size 100x30 --script <path.json> --out <path>.gif
```
`Read` both frames. Confirm:
- Four bordered panels arranged in a 2x2 grid, titled "Altitude (m)", "Velocity (m/s)", "Signal Strength (%)", "Subsystem Status".
- The three sparkline panels each show a row of visibly-varying-height block glyphs (not a flat line, not empty).
- The bar chart panel shows five labeled bars (Engines/Life Support/Comms/Nav/Power) of different lengths, all bars starting at the same column.
- Comparing the two frames, at least the sparklines' rightmost columns and the bar chart's bar lengths have visibly changed — confirming the tick-driven animation is live, not a static render.

- [ ] **Step 5: Commit**

```bash
git add examples/mission_control.rs examples/README.md
git commit -m "feat(mission-control): add ground-control telemetry example

Proves BarChart and Sparkline against a real render loop: three
animated sparklines (altitude, velocity, signal) and a subsystem-health
bar chart, all driven by a deterministic random walk."
```

---

### Task 4: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: full suite green — includes all `bar_chart::`/`sparkline::` tests from Tasks 1-2; `examples/mission_control.rs` adds no unit tests (example code, TDD-exempt).

- [ ] **Step 4: One more full `tools/visual-snapshot` capture of the finished result**

Run a capture spanning a longer window (e.g. `[{"wait_ms": 200}, {"wait_ms": 1000}, {"wait_ms": 1000}]`) to see clearer animation across all four panels. `Read` it. This is the final, whole-Arc confirmation. Reference this capture in the PR's Verification section.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo test` — full suite green, including all new `bar_chart::`/`sparkline::` tests.
- [ ] At least one `tools/visual-snapshot` capture from Task 4 is referenced in the PR description, showing the finished Mission Control console.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree` (per the documented squash-merge resolution: verify via `gh pr view --json state,mergedAt,mergeCommit`, then retry with `discard_changes: true` if the tool's own ancestry check false-positives).
