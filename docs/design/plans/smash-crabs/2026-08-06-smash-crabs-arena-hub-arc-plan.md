# Smash Crabs Arena + Hub Arc Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/2026-08-06-smash-crabs-arena-hub-arc-design.md`:
three new core widgets (`ScuttleCursor`, `DamageMeter`, `SmashBorder`)
and a full rewrite of `examples/smash_crabs.rs` from a one-screen HP
toy into a two-screen (character-select Hub → Versus arena) app with
tweened navigation, a VS-card+circle-wipe transition, real damage-%
combat feedback (particle burst + screen-shake + count-up meter, all
firing together on hit), and procedural audio cues.

**Architecture:** Seven tasks. Tasks 1-3 are core-framework (`src/widgets/`),
TDD-mandatory, independent of each other. Tasks 4-7 are all
`examples/smash_crabs.rs`, strictly sequential — **note this task order
differs from the design spec's slice numbering**: the spec listed Hub →
VS-transition → arena-polish → audio, but the VS transition's
circle-wipe needs to render a *preview* of the destination screen's
content, which for the Versus screen means it needs arena polish's
`paint_background`/`paint_ui` to already exist. So this plan builds Hub
navigation with a **plain instant screen switch** first (Task 4, same
minimal shape as Omnitrix's original Faceplate task before its own
transition was layered on), fleshes out the arena fully (Task 5), *then*
upgrades the instant switch into the real VS-card+wipe transition
(Task 6) now that there's real destination content to preview, and adds
audio last (Task 7) since its call sites live inside all three prior
tasks' handlers.

**Tech Stack:** Rust, `crossterm` (unchanged), `rodio` 0.22 (new
`[dev-dependencies]` entry — examples-only, does not touch the core
`ttui` library's own single-dependency `[dependencies]` table).

## Global Constraints

- TDD mandatory for Tasks 1-3 (`coding`-tagged, no exception applies).
  Tasks 4-7 (`examples/smash_crabs.rs`) are example code — per
  `.claude/rules/development-conventions.md`'s TDD exceptions, verified
  by running the example, not unit tested.
- Inline `#[cfg(test)] mod tests` per module — no new `tests/` directory.
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` clean after
  every task.
- No dependency changes to the `ttui` library itself (Tasks 1-3). Task 7
  adds `rodio` as a `[dev-dependencies]`-only entry.
- No RNG anywhere (particle burst angles, screen-shake offsets): all
  deterministic, matching the Omnitrix dial-navigation arc's established
  no-new-dependency-for-randomness posture.
- `ScuttleCursor`'s glyph must be single-width — do not use the vision
  doc's crab emoji (🦀 renders double-width in most terminals and would
  break this codebase's single-cell coordinate math everywhere). The
  example uses a plain ASCII `'C'`.
- Audio (Task 7) has a **stronger-than-usual** manual-verification
  caveat: this environment has no audio output device, so "verified by
  running" there means "compiles, doesn't panic, `play()` fires at the
  right call sites" — whether it actually sounds right needs you, on
  your machine, with speakers.

---

### Task 1: `ScuttleCursor` widget (`src/widgets/scuttle_cursor.rs`, #TBD)

**Files:**
- Create: `src/widgets/scuttle_cursor.rs`
- Modify: `src/widgets/mod.rs`

**Interfaces produced:**
```rust
pub struct ScuttleCursor { /* private */ }
impl ScuttleCursor {
    pub fn new(symbol: char) -> Self;
    pub fn render(&self, x: f32, y: f32, moving: bool, tick_count: u64, buf: &mut Buffer);
}
```

- [ ] **Step 1: Write the failing tests** — create `src/widgets/scuttle_cursor.rs`:

```rust
use crate::buffer::{Buffer, Cell};

pub struct ScuttleCursor {
    symbol: char,
}

impl ScuttleCursor {
    pub fn new(symbol: char) -> Self {
        ScuttleCursor { symbol }
    }

    pub fn render(&self, _x: f32, _y: f32, _moving: bool, _tick_count: u64, _buf: &mut Buffer) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_cursor_ignores_tick_count() {
        let mut buf_a = Buffer::new(10, 5);
        ScuttleCursor::new('C').render(3.4, 2.0, false, 0, &mut buf_a);
        assert_eq!(buf_a.get(3, 2).symbol, 'C');

        let mut buf_b = Buffer::new(10, 5);
        ScuttleCursor::new('C').render(3.4, 2.0, false, 99, &mut buf_b);
        assert_eq!(buf_b.get(3, 2).symbol, 'C');
    }

    #[test]
    fn moving_cursor_shifts_left_on_even_tick() {
        let mut buf = Buffer::new(10, 5);
        ScuttleCursor::new('C').render(5.0, 2.0, true, 0, &mut buf);
        assert_eq!(buf.get(4, 2).symbol, 'C');
        assert_eq!(buf.get(5, 2).symbol, ' ');
    }

    #[test]
    fn moving_cursor_shifts_right_on_odd_tick() {
        let mut buf = Buffer::new(10, 5);
        ScuttleCursor::new('C').render(5.0, 2.0, true, 1, &mut buf);
        assert_eq!(buf.get(6, 2).symbol, 'C');
        assert_eq!(buf.get(5, 2).symbol, ' ');
    }

    #[test]
    fn jerked_position_outside_bounds_does_not_panic() {
        let mut buf = Buffer::new(3, 3);
        ScuttleCursor::new('C').render(0.0, 0.0, true, 0, &mut buf);
        assert_eq!(*buf.get(0, 0), Cell::default());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::scuttle_cursor::tests`
Expected: all 4 FAIL (`not implemented`) — except the module won't even
compile as a lib target yet since it isn't registered in `mod.rs`; do
Step 5 (module registration) now so the tests can actually run, then
come back and confirm they fail on `unimplemented!()`.

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl ScuttleCursor {
    pub fn new(symbol: char) -> Self {
        ScuttleCursor { symbol }
    }

    pub fn render(&self, x: f32, y: f32, moving: bool, tick_count: u64, buf: &mut Buffer) {
        let jerk: i32 = if moving {
            if tick_count % 2 == 0 {
                -1
            } else {
                1
            }
        } else {
            0
        };
        let px = x.round() as i32 + jerk;
        let py = y.round() as i32;
        if px >= 0 && py >= 0 && (px as u16) < buf.width && (py as u16) < buf.height {
            buf.set(
                px as u16,
                py as u16,
                Cell {
                    symbol: self.symbol,
                    ..Default::default()
                },
            );
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::scuttle_cursor::tests`
Expected: all 4 PASS.

- [ ] **Step 5: Register the module** — add to `src/widgets/mod.rs`:

```rust
pub mod block;
pub mod damage_meter;
pub mod dial;
pub mod list;
pub mod scuttle_cursor;
pub mod smash_border;
pub mod table;
pub mod text;
```

(This adds all three of this arc's new modules at once — `damage_meter`
and `smash_border` don't exist as files yet, so `cargo build` will fail
until Tasks 2 and 3 create them. If you're executing tasks strictly in
order, temporarily register only `scuttle_cursor` here and add the
other two lines in Tasks 2 and 3's own module-registration steps
instead — do not leave `mod.rs` referencing files that don't exist yet.)

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/scuttle_cursor.rs src/widgets/mod.rs
git commit -m "feat(widgets): add ScuttleCursor navigation widget"
```

---

### Task 2: `DamageMeter` widget (`src/widgets/damage_meter.rs`, #TBD)

**Files:**
- Create: `src/widgets/damage_meter.rs`
- Modify: `src/widgets/mod.rs` (if not already added in Task 1's Step 5)

**Interfaces produced:**
```rust
pub struct DamageMeter { /* private */ }
impl DamageMeter {
    pub fn new(percent: u16) -> Self;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}
```

- [ ] **Step 1: Write the failing tests** — create `src/widgets/damage_meter.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct DamageMeter {
    percent: u16,
}

impl DamageMeter {
    pub fn new(percent: u16) -> Self {
        DamageMeter { percent }
    }

    pub fn render(&self, _area: Rect, _buf: &mut Buffer) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_percent_renders_white() {
        let mut buf = Buffer::new(10, 1);
        let area = Rect { x: 0, y: 0, width: 10, height: 1 };

        DamageMeter::new(0).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '0');
        assert_eq!(buf.get(1, 0).symbol, '%');
        assert_eq!(buf.get(0, 0).fg, Color::White);
    }

    #[test]
    fn fifty_percent_renders_yellow() {
        let mut buf = Buffer::new(10, 1);
        let area = Rect { x: 0, y: 0, width: 10, height: 1 };

        DamageMeter::new(50).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).fg, Color::Yellow);
    }

    #[test]
    fn over_100_percent_renders_red_with_full_text() {
        let mut buf = Buffer::new(10, 1);
        let area = Rect { x: 0, y: 0, width: 10, height: 1 };

        DamageMeter::new(137).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '1');
        assert_eq!(buf.get(1, 0).symbol, '3');
        assert_eq!(buf.get(2, 0).symbol, '7');
        assert_eq!(buf.get(3, 0).symbol, '%');
        assert_eq!(buf.get(0, 0).fg, Color::Red);
    }

    #[test]
    fn text_wider_than_area_clips_without_panic() {
        let mut buf = Buffer::new(2, 1);
        let area = Rect { x: 0, y: 0, width: 2, height: 1 };

        DamageMeter::new(137).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '1');
        assert_eq!(buf.get(1, 0).symbol, '3');
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::damage_meter::tests`
Expected: all 4 FAIL (`not implemented`).

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl DamageMeter {
    pub fn new(percent: u16) -> Self {
        DamageMeter { percent }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let color = if self.percent >= 100 {
            Color::Red
        } else if self.percent >= 50 {
            Color::Yellow
        } else {
            Color::White
        };
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::damage_meter::tests`
Expected: all 4 PASS.

- [ ] **Step 5: Confirm module registration** — `src/widgets/mod.rs` should
  now read (add `pub mod damage_meter;` if Task 1 didn't already):

```rust
pub mod block;
pub mod damage_meter;
pub mod dial;
pub mod list;
pub mod scuttle_cursor;
pub mod smash_border;
pub mod table;
pub mod text;
```

(`smash_border` still won't exist as a file until Task 3 — if building
standalone right now, comment that line out or wait for Task 3.)

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/damage_meter.rs src/widgets/mod.rs
git commit -m "feat(widgets): add DamageMeter widget"
```

---

### Task 3: `SmashBorder` widget (`src/widgets/smash_border.rs`, #TBD)

**Files:**
- Create: `src/widgets/smash_border.rs`
- Modify: `src/widgets/mod.rs` (confirm `smash_border` is registered)

**Interfaces produced:**
```rust
pub struct SmashBorder;
impl SmashBorder {
    pub fn new() -> Self;
    pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect;
}
```

**Interfaces consumed:** `Theme { background, primary, secondary,
tertiary, accent, border: BorderSet, border_bold, border_thick }`
(`src/theme.rs`, unchanged by this arc).

- [ ] **Step 1: Write the failing tests** — create `src/widgets/smash_border.rs`:

```rust
use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::theme::Theme;
use crossterm::style::Color;

pub struct SmashBorder;

impl SmashBorder {
    pub fn new() -> Self {
        SmashBorder
    }

    pub fn render(&self, area: Rect, _theme: &Theme, _buf: &mut Buffer) -> Rect {
        unimplemented!()
    }
}

impl Default for SmashBorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::BorderSet;

    fn test_theme() -> Theme {
        Theme {
            background: Color::Black,
            primary: Color::Red,
            secondary: Color::Reset,
            tertiary: Color::White,
            accent: Color::Yellow,
            border: BorderSet {
                horizontal: '=',
                vertical: '|',
                corner: '+',
            },
            border_bold: false,
            border_thick: false,
        }
    }

    #[test]
    fn draws_three_concentric_rings_and_returns_shrunk_inner_area() {
        let theme = test_theme();
        let mut buf = Buffer::new(12, 10);
        let area = Rect { x: 0, y: 0, width: 12, height: 10 };

        let inner = SmashBorder::new().render(area, &theme, &mut buf);

        assert_eq!(inner, Rect { x: 3, y: 3, width: 6, height: 4 });

        // outer ring: '#' in theme.accent
        assert_eq!(buf.get(0, 0).symbol, '#');
        assert_eq!(buf.get(0, 0).fg, Color::Yellow);
        assert_eq!(buf.get(1, 0).symbol, '#');
        assert_eq!(buf.get(1, 0).fg, Color::Yellow);

        // middle ring: theme.border glyphs in theme.primary
        assert_eq!(buf.get(1, 1).symbol, '+');
        assert_eq!(buf.get(1, 1).fg, Color::Red);
        assert_eq!(buf.get(2, 1).symbol, '=');
        assert_eq!(buf.get(2, 1).fg, Color::Red);

        // inner ring: '-'/':'/'.' in theme.tertiary
        assert_eq!(buf.get(2, 2).symbol, '.');
        assert_eq!(buf.get(2, 2).fg, Color::White);
        assert_eq!(buf.get(3, 2).symbol, '-');
        assert_eq!(buf.get(3, 2).fg, Color::White);
    }

    #[test]
    fn too_small_area_degrades_gracefully_without_panic() {
        let theme = test_theme();
        let mut buf = Buffer::new(3, 3);
        let area = Rect { x: 0, y: 0, width: 3, height: 3 };

        let inner = SmashBorder::new().render(area, &theme, &mut buf);

        assert_eq!(inner, Rect { x: 1, y: 1, width: 1, height: 1 });
        assert_eq!(*buf.get(1, 1), Cell::default());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::smash_border::tests`
Expected: both FAIL (`not implemented`).

- [ ] **Step 3: Implement** — replace the `render` method body:

```rust
impl SmashBorder {
    pub fn new() -> Self {
        SmashBorder
    }

    pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect {
        let rings: [(char, char, char, Color); 3] = [
            ('#', '#', '#', theme.accent),
            (
                theme.border.horizontal,
                theme.border.vertical,
                theme.border.corner,
                theme.primary,
            ),
            ('-', ':', '.', theme.tertiary),
        ];

        let mut inner = area;
        for (h, v, c, color) in rings {
            if inner.width < 2 || inner.height < 2 {
                break;
            }
            for x in inner.x..inner.x + inner.width {
                buf.set(
                    x,
                    inner.y,
                    Cell {
                        symbol: h,
                        fg: color,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
                buf.set(
                    x,
                    inner.y + inner.height - 1,
                    Cell {
                        symbol: h,
                        fg: color,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
            for y in inner.y..inner.y + inner.height {
                buf.set(
                    inner.x,
                    y,
                    Cell {
                        symbol: v,
                        fg: color,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
                buf.set(
                    inner.x + inner.width - 1,
                    y,
                    Cell {
                        symbol: v,
                        fg: color,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
            buf.set(
                inner.x,
                inner.y,
                Cell {
                    symbol: c,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x + inner.width - 1,
                inner.y,
                Cell {
                    symbol: c,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x,
                inner.y + inner.height - 1,
                Cell {
                    symbol: c,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x + inner.width - 1,
                inner.y + inner.height - 1,
                Cell {
                    symbol: c,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );

            inner = Rect {
                x: inner.x + 1,
                y: inner.y + 1,
                width: inner.width.saturating_sub(2),
                height: inner.height.saturating_sub(2),
            };
        }

        inner
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::smash_border::tests`
Expected: both PASS.

- [ ] **Step 5: Confirm module registration** — `src/widgets/mod.rs`
  should now read exactly:

```rust
pub mod block;
pub mod damage_meter;
pub mod dial;
pub mod list;
pub mod scuttle_cursor;
pub mod smash_border;
pub mod table;
pub mod text;
```

- [ ] **Step 6: Run full check**

Run: `cargo test --lib && cargo build --examples && cargo fmt && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings. (`cargo build --examples` should still
succeed unchanged — nothing in `examples/` uses the three new widgets
yet.)

- [ ] **Step 7: Commit**

```bash
git add src/widgets/smash_border.rs src/widgets/mod.rs
git commit -m "feat(widgets): add SmashBorder widget"
```

---

### Task 4: Hub screen + instant screen switching (`examples/smash_crabs.rs`)

**Files:**
- Modify: `examples/smash_crabs.rs`

**Interfaces consumed:** `ScuttleCursor::new(char).render(x, y, moving,
tick_count, buf)` (Task 1); `SmashBorder::new().render(area, theme, buf)
-> Rect` (Task 3); `Transition::start/tick/progress/is_complete`
(`src/transition.rs`, unchanged); `easing::{lerp, ease_out}`
(`src/easing.rs`, unchanged).

**Interfaces produced:** none public — everything in this file is
private to the example binary. Internal shape other tasks build on:
`Screen` enum, `FIGHTERS` const, `SmashCrabs.screen/selected/
cursor_tween` fields, `screen_for_selected(usize) -> Screen`.

No new tests — example code, verified by running.

- [ ] **Step 1: Update imports** — replace the top of the file:

```rust
// examples/smash_crabs.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::easing;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{
    block::Block, scuttle_cursor::ScuttleCursor, smash_border::SmashBorder, text::Text,
};
```

- [ ] **Step 2: Add the `Screen` enum, `FIGHTERS` const, and
  `screen_for_selected` helper** — insert above `fn arena_theme()`:

```rust
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Hub,
    Versus,
    TargetSmash,
    StageHazards,
}

const FIGHTERS: [&str; 3] = ["Versus Mode", "Target Smash", "Stage Hazards"];
const CURSOR_TWEEN_MS: u64 = 150;
const CURSOR_SYMBOL: char = 'C';

fn screen_for_selected(selected: usize) -> Screen {
    match selected {
        0 => Screen::Versus,
        1 => Screen::TargetSmash,
        _ => Screen::StageHazards,
    }
}
```

- [ ] **Step 3: Add Hub-related fields to `SmashCrabs`** — change the
  struct definition (currently `theme`/`p1_hp`/`p2_hp`/
  `flash_ticks_remaining`/`quit`):

```rust
struct SmashCrabs {
    theme: Theme,
    screen: Screen,
    selected: usize,
    cursor_tween: Option<(f32, Transition)>,
    p1_hp: u8,
    p2_hp: u8,
    flash_ticks_remaining: u8,
    quit: bool,
}
```

  and the corresponding fields in `new()`'s `SmashCrabs { .. }` literal:

```rust
impl SmashCrabs {
    fn new() -> Self {
        SmashCrabs {
            theme: arena_theme(),
            screen: Screen::Hub,
            selected: 0,
            cursor_tween: None,
            p1_hp: 100,
            p2_hp: 100,
            flash_ticks_remaining: 0,
            quit: false,
        }
    }
```

  (Leave `paint_background`/`paint_ui`/`paint_effects` exactly as they
  are for this task — untouched, still writing directly into
  `buf.layer_mut(N)`. Task 5 rewrites them.)

- [ ] **Step 4: Add Hub navigation/rendering helpers** — add these
  methods to `impl SmashCrabs` (alongside `paint_background` etc.):

```rust
    fn displayed_cursor_index(&self) -> f32 {
        match &self.cursor_tween {
            Some((from, t)) => easing::ease_out(*from, self.selected as f32, t.progress()),
            None => self.selected as f32,
        }
    }

    fn hub_panels(area: Rect) -> Vec<Rect> {
        Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); FIGHTERS.len()]).split(area)
    }

    fn cursor_position(&self, area: Rect) -> (f32, f32) {
        let panels = Self::hub_panels(area);
        let centers: Vec<f32> = panels
            .iter()
            .map(|p| p.x as f32 + p.width as f32 / 2.0)
            .collect();
        let index = self.displayed_cursor_index();
        let lo = (index.floor() as usize).min(centers.len() - 1);
        let hi = (lo + 1).min(centers.len() - 1);
        let frac = index - lo as f32;
        let x = easing::lerp(centers[lo], centers[hi], frac);
        let y = area.y as f32 + area.height as f32 - 2.0;
        (x, y)
    }

    fn render_hub(&self, area: Rect, buf: &mut LayerStack) {
        let inner = SmashBorder::new().render(area, &self.theme, buf);
        let panels = Self::hub_panels(inner);
        for (i, panel) in panels.iter().enumerate() {
            let name_row = Rect {
                x: panel.x,
                y: panel.y,
                width: panel.width,
                height: panel.height.min(1),
            };
            Text::new(FIGHTERS[i]).render(name_row, buf);
        }
        let (cx, cy) = self.cursor_position(inner);
        ScuttleCursor::new(CURSOR_SYMBOL).render(
            cx,
            cy,
            self.cursor_tween.is_some(),
            0,
            buf,
        );
        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new("Left/Right move * Enter select * q quit").render(hint_row, buf);
    }

    fn render_placeholder(&self, screen: Screen, area: Rect, buf: &mut LayerStack) {
        let inner = SmashBorder::new().render(area, &self.theme, buf);
        let name = match screen {
            Screen::TargetSmash => "Target Smash",
            Screen::StageHazards => "Stage Hazards",
            _ => "",
        };
        let name_row = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.min(1),
        };
        let placeholder_row = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: inner.height.saturating_sub(2),
        };
        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new(name).render(name_row, buf);
        Text::new("(not yet built)").render(placeholder_row, buf);
        Text::new("Esc back * q quit").render(hint_row, buf);
    }
```

  Note: `render_hub` passes a hardcoded `0` for `ScuttleCursor::render`'s
  `tick_count` parameter for now — there's no tick counter on
  `SmashCrabs` yet in this task, so the jerk always resolves to the
  "even tick" (`-1`) offset while moving. Task 5 introduces a real
  `tick_count` field (needed for particle/shake determinism anyway) and
  Task 5's own steps thread it through here too. This is a known,
  temporary, self-contained simplification — the cursor still tweens
  smoothly via `cursor_position`'s eased `x`, it just won't alternate
  its jerk direction until Task 5.

- [ ] **Step 5: Update `update()`** — replace the method body:

```rust
impl App for SmashCrabs {
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
                    let from = self.displayed_cursor_index();
                    self.selected = (self.selected + FIGHTERS.len() - 1) % FIGHTERS.len();
                    self.cursor_tween =
                        Some((from, Transition::start(Duration::from_millis(CURSOR_TWEEN_MS))));
                }
                KeyCode::Right => {
                    let from = self.displayed_cursor_index();
                    self.selected = (self.selected + 1) % FIGHTERS.len();
                    self.cursor_tween =
                        Some((from, Transition::start(Duration::from_millis(CURSOR_TWEEN_MS))));
                }
                KeyCode::Enter => {
                    if self.cursor_tween.is_none() {
                        self.screen = screen_for_selected(self.selected);
                    }
                }
                _ => {}
            },
            Screen::Versus => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                } else if k.code == KeyCode::Char(' ') {
                    self.flash_ticks_remaining = FLASH_TICKS;
                    self.p2_hp = self.p2_hp.saturating_sub(10);
                }
            }
            Screen::TargetSmash | Screen::StageHazards => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                }
            }
        }
    }
```

  (The `Screen::Versus` arm body here is the pre-existing hit logic,
  untouched — Task 5 replaces it with the full combo.)

- [ ] **Step 6: Update `view()`** — replace the method body:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        match self.screen {
            Screen::Hub => self.render_hub(area, buf),
            Screen::Versus => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.paint_background(area, buf);
                self.paint_ui(area, buf);
                self.paint_effects(area, buf);
            }
            Screen::TargetSmash | Screen::StageHazards => {
                self.render_placeholder(self.screen, area, buf)
            }
        }
    }
```

- [ ] **Step 7: Add cursor-tween ticking to `on_tick`** — append to the
  end of the existing `on_tick` body (after the existing
  `flash_ticks_remaining` decrement, before the closing brace):

```rust
        if let Some((_, t)) = &mut self.cursor_tween {
            t.tick(elapsed);
            if t.is_complete() {
                self.cursor_tween = None;
            }
        }
    }
}
```

- [ ] **Step 8: Build**

Run: `cargo build --example smash_crabs`
Expected: compiles cleanly, no warnings. (Watch for an unused-import
warning on `Block` — if `paint_ui` is the only remaining user and it's
untouched from before, `Block` should still be in use; if the compiler
flags it unused, you've removed a call that still needs it — investigate
rather than deleting the import.)

- [ ] **Step 9: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 10: Manual verification** (real-terminal check, not
  automatable — per this project's TDD exceptions for example code):

Run: `cargo run --example smash_crabs`

Confirm:
- App opens on the Hub showing 3 panels ("Versus Mode", "Target Smash",
  "Stage Hazards") inside a 3-ring `SmashBorder` frame, with the `C`
  cursor under the first panel.
- `Right`/`Left` move the cursor between panels with a visible tween
  (not an instant jump) and wrap around at the ends.
- `Enter` on each panel switches instantly (no transition yet — that's
  Task 6) to that screen: Versus Mode shows the existing arena, Target
  Smash/Stage Hazards show their name + "(not yet built)".
- `Esc` from any of the three destination screens returns to the Hub
  with the same panel still selected.
- The arena's existing behavior (Space bar flashes + drops P2 HP) still
  works unchanged.
- `q` quits cleanly from every screen, no panic, no leftover terminal
  attributes.

- [ ] **Step 11: Commit**

```bash
git add examples/smash_crabs.rs
git commit -m "feat(smash_crabs): add character-select Hub with instant screen switching"
```

---

### Task 5: Arena polish (`examples/smash_crabs.rs`)

**Files:**
- Modify: `examples/smash_crabs.rs`

**Interfaces consumed:** `DamageMeter::new(u16).render(area, buf)`
(Task 2); `SmashBorder::new().render(area, theme, buf) -> Rect`
(Task 3); `effects::shake(&Buffer, i16, i16) -> Buffer` (`src/effects.rs`,
unchanged); `ParticleSystem::{new, spawn, update, render}` and
`Particle { x, y, vx, vy, symbol, color, lifetime, age }`
(`src/particles.rs`, unchanged); `easing::ease_out` (unchanged).

**Interfaces produced:** `SmashCrabs.p2_damage: u16` replaces `p1_hp`/
`p2_hp`; `SmashCrabs.tick_count: u64` (new — also fixes Task 4's
hardcoded `0` passed to `ScuttleCursor::render`); a `blit` free function
reused by Task 6.

No new tests — example code, verified by running.

- [ ] **Step 1: Update imports** — add to the top of the file:

```rust
use ttui::buffer::{Buffer, Cell, LayerStack};
use ttui::effects;
use ttui::particles::{Particle, ParticleSystem};
use ttui::widgets::{
    block::Block, damage_meter::DamageMeter, scuttle_cursor::ScuttleCursor,
    smash_border::SmashBorder, text::Text,
};
```

  (`Block` is no longer used anywhere after this task's Step 3 rewrites
  `paint_ui` to use `SmashBorder` instead — remove the `block::Block`
  import entirely once Step 3 is done and `cargo build` flags it
  unused. `Buffer`/`Cell` are newly needed for the scratch-buffer
  pattern below.)

- [ ] **Step 2: Replace HP fields with damage/shake/particle/tick
  state** — change the struct definition:

```rust
struct SmashCrabs {
    theme: Theme,
    screen: Screen,
    selected: usize,
    cursor_tween: Option<(f32, Transition)>,
    p2_damage: u16,
    damage_tween: Option<(f32, Transition)>,
    flash_ticks_remaining: u8,
    shake_ticks_remaining: u8,
    particles: ParticleSystem,
    tick_count: u64,
    quit: bool,
}
```

  and the corresponding fields in `new()`:

```rust
impl SmashCrabs {
    fn new() -> Self {
        SmashCrabs {
            theme: arena_theme(),
            screen: Screen::Hub,
            selected: 0,
            cursor_tween: None,
            p2_damage: 0,
            damage_tween: None,
            flash_ticks_remaining: 0,
            shake_ticks_remaining: 0,
            particles: ParticleSystem::new(),
            tick_count: 0,
            quit: false,
        }
    }
```

  Add the new constants near `FLASH_TICKS`:

```rust
const FLASH_TICKS: u8 = 6; // ~200ms flash at 33ms/tick
const SHAKE_TICKS: u8 = 6; // matches FLASH_TICKS's ~200ms feel
const DAMAGE_TWEEN_MS: u64 = 250;
const HIT_DAMAGE: u16 = 17;
```

- [ ] **Step 3: Rewrite `paint_background`/`paint_ui`/`paint_effects`
  to return scratch buffers** — replace all three methods:

```rust
    fn displayed_p2_damage(&self) -> f32 {
        match &self.damage_tween {
            Some((from, t)) => easing::ease_out(*from, self.p2_damage as f32, t.progress()),
            None => self.p2_damage as f32,
        }
    }

    fn shake_offset(&self) -> (i16, i16) {
        if self.shake_ticks_remaining == 0 {
            return (0, 0);
        }
        let magnitude = (((self.shake_ticks_remaining as i16) + 1) / 2).min(2);
        let dx = if self.shake_ticks_remaining % 2 == 0 {
            magnitude
        } else {
            -magnitude
        };
        let dy = if (self.shake_ticks_remaining / 2) % 2 == 0 {
            magnitude
        } else {
            -magnitude
        };
        (dx, dy)
    }

    fn paint_background(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let cell = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            ..Default::default()
        };
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(x, y, cell.clone());
            }
        }
        buf
    }

    fn paint_ui(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let panel = Layout::new(Direction::Vertical, vec![Constraint::Fixed(8)]).split(local)[0];
        let panel = Rect {
            width: panel.width.min(24),
            ..panel
        };
        let inner = SmashBorder::new().render(panel, &self.theme, &mut buf);
        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fixed(1)],
        )
        .split(inner);
        DamageMeter::new(0).render(rows[0], &mut buf);
        DamageMeter::new(self.displayed_p2_damage().round() as u16).render(rows[1], &mut buf);
        buf
    }

    fn paint_effects(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        if self.flash_ticks_remaining > 0 {
            let flash = Cell {
                symbol: '*',
                fg: Color::Black,
                bg: self.theme.accent,
                ..Default::default()
            };
            let w = 7.min(area.width);
            let h = 3.min(area.height);
            let x0 = (area.width.saturating_sub(w)) / 2;
            let y0 = (area.height.saturating_sub(h)) / 2;
            for y in y0..y0 + h {
                for x in x0..x0 + w {
                    buf.set(x, y, flash.clone());
                }
            }
        }
        self.particles.render(&mut buf);
        buf
    }

    fn render_versus(&self, area: Rect, buf: &mut LayerStack) {
        let (dx, dy) = self.shake_offset();
        let layers: [(usize, Buffer); 3] = [
            (BACKGROUND, self.paint_background(area)),
            (UI, self.paint_ui(area)),
            (EFFECTS, self.paint_effects(area)),
        ];
        for (index, scratch) in layers {
            let final_buf = if dx != 0 || dy != 0 {
                effects::shake(&scratch, dx, dy)
            } else {
                scratch
            };
            blit(&final_buf, area, buf.layer_mut(index));
        }
    }
```

  Note the P1 row uses a static `DamageMeter::new(0)` — per the design
  spec, this arc's Space-bar interaction is one-directional (P1 hits
  P2), so P1's meter has no live value to display yet; it's still shown
  via `DamageMeter` for visual consistency with P2's row.

- [ ] **Step 4: Add the `blit` free function** — add at module scope
  (below the `impl App for SmashCrabs` block, above `fn main`):

```rust
fn blit(scratch: &Buffer, area: Rect, buf: &mut Buffer) {
    for y in 0..scratch.height {
        for x in 0..scratch.width {
            buf.set(area.x + x, area.y + y, scratch.get(x, y).clone());
        }
    }
}
```

- [ ] **Step 5: Replace the Space-bar hit handler with the full combo**
  — in `update()`, replace the `Screen::Versus` arm's `Char(' ')` branch:

```rust
            Screen::Versus => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Hub;
                } else if k.code == KeyCode::Char(' ') {
                    self.flash_ticks_remaining = FLASH_TICKS;
                    self.shake_ticks_remaining = SHAKE_TICKS;
                    let from = self.displayed_p2_damage();
                    self.p2_damage += HIT_DAMAGE;
                    self.damage_tween =
                        Some((from, Transition::start(Duration::from_millis(DAMAGE_TWEEN_MS))));
                    for i in 0..8 {
                        let angle = i as f32 * std::f32::consts::TAU / 8.0;
                        self.particles.spawn(Particle {
                            x: 10.0,
                            y: 4.0,
                            vx: angle.cos() * 8.0,
                            vy: angle.sin() * 4.0,
                            symbol: '*',
                            color: self.theme.accent,
                            lifetime: Duration::from_millis(400),
                            age: Duration::ZERO,
                        });
                    }
                }
            }
```

  The particle spawn point `(10.0, 4.0)` is a fixed local coordinate
  approximating the UI panel's location (the panel is up to 24 wide by
  8 tall, positioned at the arena's top-left) — `update()` doesn't
  receive the render `area` (the `App` trait doesn't pass it), so this
  is a deliberate fixed approximation rather than a precisely-computed
  panel-relative point. Acceptable for this arc's scope; documented here
  so it isn't mistaken for an oversight.

- [ ] **Step 6: Update `view()`'s `Screen::Versus` arm** — replace:

```rust
            Screen::Versus => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.paint_background(area, buf);
                self.paint_ui(area, buf);
                self.paint_effects(area, buf);
            }
```

  with:

```rust
            Screen::Versus => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.render_versus(area, buf);
            }
```

- [ ] **Step 7: Thread real `tick_count` into `render_hub`'s
  `ScuttleCursor::render` call** — in `render_hub` (added in Task 4),
  change:

```rust
        ScuttleCursor::new(CURSOR_SYMBOL).render(
            cx,
            cy,
            self.cursor_tween.is_some(),
            0,
            buf,
        );
```

  to:

```rust
        ScuttleCursor::new(CURSOR_SYMBOL).render(
            cx,
            cy,
            self.cursor_tween.is_some(),
            self.tick_count,
            buf,
        );
```

- [ ] **Step 8: Tick shake/damage-tween/particles/tick_count in
  `on_tick`** — append to the end of the existing `on_tick` body
  (after the `cursor_tween` block Task 4 added, before the closing
  brace):

```rust
        self.tick_count += 1;

        if let Some((_, t)) = &mut self.damage_tween {
            t.tick(elapsed);
            if t.is_complete() {
                self.damage_tween = None;
            }
        }

        if self.shake_ticks_remaining > 0 {
            self.shake_ticks_remaining -= 1;
        }

        self.particles.update(elapsed);
    }
}
```

- [ ] **Step 9: Build**

Run: `cargo build --example smash_crabs`
Expected: compiles cleanly. If `block::Block` is now flagged unused,
remove it from the import list (Step 1's note above).

- [ ] **Step 10: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 11: Manual verification**

Run: `cargo run --example smash_crabs`

Confirm, in the Versus Mode screen:
- The "Fighters" panel now shows two `DamageMeter` rows ("0%" for P1,
  starting "0%" for P2) inside a visibly 3-ringed `SmashBorder` frame
  (distinct outer/middle/inner glyphs and colors) instead of the old
  plain `Block`.
- Pressing Space: P2's damage counts up (not down) with a visible
  count-up animation rather than an instant jump, an 8-point particle
  burst radiates from the panel area and fades over ~400ms, the whole
  screen visibly shakes for a few frames, and the existing yellow flash
  still plays — all four happening together on one keypress.
- Repeated rapid Space presses don't panic or visually break (damage
  tween restarting mid-animation should look like a smooth redirect, not
  a snap).
- Back in the Hub, the cursor's jerky left/right shift while moving is
  now visible (Task 4's temporary always-left jerk is fixed).
- `q` still quits cleanly from every screen.

- [ ] **Step 12: Commit**

```bash
git add examples/smash_crabs.rs
git commit -m "feat(smash_crabs): wire Arc 0 primitives into arena (shake, particles, damage tween)"
```

---

### Task 6: VS transition (`examples/smash_crabs.rs`)

**Files:**
- Modify: `examples/smash_crabs.rs`

**Interfaces consumed:** `render_hub`, `render_placeholder`,
`paint_background`, `paint_ui` (all from Tasks 4-5, now needed to build
a destination-screen preview); `blit` (Task 5); `Transition`/`CellStyle`
(unchanged).

No new tests — example code, verified by running.

- [ ] **Step 1: Add `CellStyle` to the buffer import** — change:

```rust
use ttui::buffer::{Buffer, Cell, LayerStack};
```

  to:

```rust
use ttui::buffer::{Buffer, Cell, CellStyle, LayerStack};
```

- [ ] **Step 2: Add transition state** — change the struct definition:

```rust
struct SmashCrabs {
    theme: Theme,
    screen: Screen,
    selected: usize,
    cursor_tween: Option<(f32, Transition)>,
    transitioning_to: Option<(Screen, Transition)>,
    p2_damage: u16,
    damage_tween: Option<(f32, Transition)>,
    flash_ticks_remaining: u8,
    shake_ticks_remaining: u8,
    particles: ParticleSystem,
    tick_count: u64,
    quit: bool,
}
```

  and the corresponding field in `new()`:

```rust
            cursor_tween: None,
            transitioning_to: None,
            p2_damage: 0,
```

  Add the new constant near `CURSOR_TWEEN_MS`:

```rust
const VS_TRANSITION_MS: u64 = 700;
```

- [ ] **Step 3: Replace the Hub's instant `Enter` switch with a
  transition start** — in `update()`'s `Screen::Hub` arm, replace:

```rust
                KeyCode::Enter => {
                    if self.cursor_tween.is_none() {
                        self.screen = screen_for_selected(self.selected);
                    }
                }
```

  with:

```rust
                KeyCode::Enter => {
                    if self.cursor_tween.is_none() {
                        let destination = screen_for_selected(self.selected);
                        self.transitioning_to = Some((
                            destination,
                            Transition::start(Duration::from_millis(VS_TRANSITION_MS)),
                        ));
                        self.p2_damage = 0;
                        self.damage_tween = None;
                        self.flash_ticks_remaining = 0;
                        self.shake_ticks_remaining = 0;
                        self.particles = ParticleSystem::new();
                    }
                }
```

  (Resetting arena state here — not when the transition completes —
  means both the transition's mid-flight preview and the eventual live
  arena start from a clean slate, since `self.p2_damage` etc. are
  already `0`/`None` by the time anything renders them.)

- [ ] **Step 4: Ignore navigation input while transitioning** — in
  `update()`, add a guard right after the `q` check (before the
  `match self.screen` block):

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
  to the end of `on_tick` (after the `tick_count += 1` line Task 5
  added, anywhere before the closing brace):

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
  helpers** — add these methods to `impl SmashCrabs`:

```rust
    fn render_destination_preview(&self, screen: Screen, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        match screen {
            Screen::Versus => {
                let background = self.paint_background(local);
                blit(&background, local, &mut buf);
                let ui = self.paint_ui(local);
                blit(&ui, local, &mut buf);
            }
            Screen::TargetSmash | Screen::StageHazards => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_placeholder(screen, local, &mut stack);
                blit(&stack, local, &mut buf);
            }
            Screen::Hub => {
                let mut stack = LayerStack::new(area.width, area.height);
                self.render_hub(local, &mut stack);
                blit(&stack, local, &mut buf);
            }
        }
        buf
    }

    fn render_transition(&self, destination: Screen, area: Rect, progress: f32, buf: &mut Buffer) {
        if progress < 0.4 {
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
            let label = "VS";
            let lx = area.x + area.width.saturating_sub(label.len() as u16) / 2;
            let ly = area.y + area.height / 2;
            for (i, ch) in label.chars().enumerate() {
                buf.set(
                    lx + i as u16,
                    ly,
                    Cell {
                        symbol: ch,
                        fg: Color::White,
                        bg: Color::Black,
                        style: CellStyle { bold: true },
                    },
                );
            }
            return;
        }

        let wipe = (progress - 0.4) / 0.6;
        let content = self.render_destination_preview(destination, area);
        let cx = area.width as f32 / 2.0;
        let cy = area.height as f32 / 2.0;
        let max_radius = ((cx / 2.0).powi(2) + cy.powi(2)).sqrt();
        let radius = wipe * max_radius;
        for y in 0..area.height {
            for x in 0..area.width {
                let dx = (x as f32 - cx) / 2.0;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let cell = if dist <= radius {
                    content.get(x, y).clone()
                } else {
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: Color::Black,
                        ..Default::default()
                    }
                };
                buf.set(area.x + x, area.y + y, cell);
            }
        }
    }
```

  Note `render_destination_preview`'s `TargetSmash`/`StageHazards`/`Hub`
  arms build a throwaway `LayerStack` to call the existing
  `&mut LayerStack`-taking `render_placeholder`/`render_hub` methods,
  then `blit` its base layer into the flat scratch `Buffer` — reusing
  those methods as-is rather than duplicating their drawing logic. The
  `Hub` arm is included for completeness (`Screen` is exhaustively
  matched) even though nothing in this arc ever transitions *into* the
  Hub (`Esc` returns instantly, no transition).

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
            Screen::Versus => {
                buf.push_layer(); // index 1: UI
                buf.push_layer(); // index 2: EFFECTS
                self.render_versus(area, buf);
            }
            Screen::TargetSmash | Screen::StageHazards => {
                self.render_placeholder(self.screen, area, buf)
            }
        }
    }
```

- [ ] **Step 8: Build**

Run: `cargo build --example smash_crabs`
Expected: compiles cleanly.

- [ ] **Step 9: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 10: Manual verification**

Run: `cargo run --example smash_crabs`

Confirm, selecting each of the 3 Hub panels in turn:
- Pressing `Enter` cuts to a solid black screen with a bold white "VS"
  centered, holding briefly.
- The screen then wipes in from the center outward in a genuinely
  circular (not elliptical/oval) shape, revealing the destination
  screen's content as the circle expands, until it fills the screen.
- Left/Right/Enter/Esc are all ignored while the transition plays; `q`
  still quits immediately even mid-transition.
- Landing in Versus Mode: the arena appears with damage already reset
  to 0% (no stale value from a previous visit) if you'd hit Space
  before returning to the Hub earlier.
- `q` quits cleanly from every state, no panic, no leftover terminal
  attributes.

- [ ] **Step 11: Commit**

```bash
git add examples/smash_crabs.rs
git commit -m "feat(smash_crabs): add VS-card and circle-wipe transition"
```

---

### Task 7: Audio cues (`examples/smash_crabs.rs`, `Cargo.toml`)

**Files:**
- Modify: `examples/smash_crabs.rs`
- Modify: `Cargo.toml`

**Interfaces consumed:** `ttui::audio::AudioSink` (`src/audio.rs`,
unchanged: `trait AudioSink { fn play(&mut self, event_id: &str); }`).
`rodio::stream::{DeviceSinkBuilder, MixerDeviceSink}`,
`rodio::source::{SineWave, Source}` — rodio 0.22 API, confirmed against
published docs.rs pages during design, not compiled locally; treat any
compile mismatch here as an expected minor fixup against real compiler
output, not a sign of a wrong design.

**Interfaces produced:** none public.

No new tests — example code, verified by running; audio playback itself
cannot be verified in this environment (no audio device) — see the
Global Constraints note.

- [ ] **Step 1: Add the dev-dependency** — in `Cargo.toml`, add a new
  table after `[dependencies]`:

```toml
[dependencies]
crossterm = "0.27"

[dev-dependencies]
rodio = "0.22"
```

- [ ] **Step 2: Add the `AudioSink` and `RodioAudioSink` imports** —
  add to the top of `examples/smash_crabs.rs`:

```rust
use ttui::audio::AudioSink;
```

- [ ] **Step 3: Add `RodioAudioSink`** — add above `struct SmashCrabs`:

```rust
struct RodioAudioSink {
    sink: Option<rodio::stream::MixerDeviceSink>,
}

impl RodioAudioSink {
    fn new() -> Self {
        match rodio::stream::DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => RodioAudioSink { sink: Some(sink) },
            Err(_) => RodioAudioSink { sink: None },
        }
    }
}

impl AudioSink for RodioAudioSink {
    fn play(&mut self, event_id: &str) {
        let Some(sink) = &self.sink else { return };
        let freq: f32 = match event_id {
            "cursor" => 440.0,
            "select" => 660.0,
            "hit" => 220.0,
            _ => return,
        };
        let source = rodio::source::SineWave::new(freq)
            .take_duration(Duration::from_millis(120))
            .amplify(0.2);
        sink.mixer().add(source);
    }
}
```

  If `rodio::source::SineWave`/`Source::take_duration`/`Source::amplify`
  don't resolve exactly as written once real docs are available in this
  checkout (`cargo doc --open -p rodio` or `cargo build` errors will
  say precisely what's wrong), adjust the import path and method chain
  to match — the *intent* (a short, quiet, fixed-frequency tone per
  event, silently no-op'd when no output device exists) is the actual
  requirement, not these exact tokens.

- [ ] **Step 4: Add the `audio` field to `SmashCrabs`** — change the
  struct definition (append after `tick_count`):

```rust
struct SmashCrabs {
    theme: Theme,
    screen: Screen,
    selected: usize,
    cursor_tween: Option<(f32, Transition)>,
    transitioning_to: Option<(Screen, Transition)>,
    p2_damage: u16,
    damage_tween: Option<(f32, Transition)>,
    flash_ticks_remaining: u8,
    shake_ticks_remaining: u8,
    particles: ParticleSystem,
    tick_count: u64,
    audio: RodioAudioSink,
    quit: bool,
}
```

  and in `new()`:

```rust
            tick_count: 0,
            audio: RodioAudioSink::new(),
            quit: false,
        }
    }
```

- [ ] **Step 5: Wire the three call sites** — in `update()`:

  - Hub `Left` arm, after setting `cursor_tween`, add:
    ```rust
    self.audio.play("cursor");
    ```
  - Hub `Right` arm, after setting `cursor_tween`, add the same line.
  - Hub `Enter` arm, after resetting arena state (inside the
    `if self.cursor_tween.is_none()` block, after `self.particles =
    ParticleSystem::new();`), add:
    ```rust
    self.audio.play("select");
    ```
  - Versus `Char(' ')` arm (the hit combo), after spawning the particle
    burst loop, add:
    ```rust
    self.audio.play("hit");
    ```

  `update()` takes `&mut self`, and `AudioSink::play` takes `&mut self`
  on `RodioAudioSink` — no borrow conflicts, since `self.audio.play(...)`
  borrows only the `audio` field mutably, disjoint from the other field
  writes around it.

- [ ] **Step 6: Build**

Run: `cargo build --example smash_crabs`
Expected: compiles cleanly. This is the first build to resolve `rodio`
and its transitive dependency tree (`cpal` etc.) — allow extra time for
the initial fetch/compile; if it fails, read the actual error before
assuming the design is wrong (see Step 3's note).

- [ ] **Step 7: `cargo fmt && cargo clippy --all-targets -- -D warnings`**

Expected: clean, no warnings.

- [ ] **Step 8: Manual verification** (real-terminal check with audio
  hardware — cannot be performed by the implementing agent in a
  headless environment; this step requires you specifically):

Run: `cargo run --example smash_crabs`

Confirm:
- Moving the Hub cursor plays a short click-like tone.
- Selecting a panel (`Enter`) plays a distinct higher tone right as the
  VS transition starts.
- Hitting Space in the Arena plays a distinct lower "impact" tone
  alongside the shake/particles/flash/damage-count-up.
- No audio device present (or audio failing for any reason) does not
  crash the app — if you want to confirm this specifically, there's no
  clean way to simulate "no device" without physically disabling audio
  output, so take the `Err(_) => sink: None` fallback on faith from the
  code unless you hit a real crash.
- `q` still quits cleanly from every state.

- [ ] **Step 9: Commit**

```bash
git add examples/smash_crabs.rs Cargo.toml Cargo.lock
git commit -m "feat(smash_crabs): add procedural audio cues via rodio"
```

---

## Self-Review

**Spec coverage:** `ScuttleCursor` (jerk rendering, single-width-glyph
caveat) — Task 1. `DamageMeter` (thresholds, clipping) — Task 2.
`SmashBorder` (3 inward rings, graceful degradation) — Task 3. Hub
screen (3-panel grid, tweened cursor, Left/Right/Enter/Esc/q) — Task 4.
Arena polish (damage tween, particle burst, screen-shake applied
uniformly across all 3 layers, `DamageMeter`/`SmashBorder` wiring) —
Task 5. VS transition (black+"VS" card, circular wipe with aspect-ratio
correction, input ignored mid-transition) — Task 6. Audio cues
(procedural tones, graceful no-device fallback, 3 call sites) — Task 7.
Verification section (`cargo test`/`fmt`/`clippy` + manual
`cargo run --example smash_crabs` walkthrough) — covered across every
task's final steps. The spec's explicit out-of-scope list (real
TargetSmash/StageHazards content, full 4x3/5x4 grid, bundled audio
assets, boot splash, P1 taking damage, `Theme`/`Block` changes) — none
added anywhere in this plan.

**Placeholder scan:** no TBD/TODO in code or commands. Task 4's
hardcoded `0` tick_count is explicitly flagged as a known, temporary,
self-documented simplification with the exact task (5) that fixes it
named — not an unresolved placeholder. Task 7's rodio API is flagged as
design-time-verified-not-compile-verified, with explicit guidance on
what to do if it's wrong, rather than asserted as certain.

**Type consistency:** `Screen`, `FIGHTERS`, `screen_for_selected`
(Task 4) are used identically in Tasks 5-7 — no renames. `ScuttleCursor::
render`'s 5-argument signature (Task 1) matches exactly how Task 4's
`render_hub` calls it (initially with a literal `0`, corrected to
`self.tick_count` in Task 5 once that field exists — both are valid
calls against the same unchanged signature, not a signature change).
`SmashBorder::render(area, theme, buf) -> Rect` (Task 3) matches its
every call site (Tasks 4, 5, 6's `render_hub`/`render_placeholder`/
`paint_ui`). `p2_damage`/`damage_tween`/`shake_ticks_remaining`/
`particles`/`tick_count` are introduced once in Task 5 and read/written
identically in Tasks 6-7. `blit` (Task 5) is reused verbatim in Task 6
with no signature drift.
