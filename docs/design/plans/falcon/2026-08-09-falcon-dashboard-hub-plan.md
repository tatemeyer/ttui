# Falcon Dashboard Hub Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the fourth themed example app's hub — a 3-panel smuggler-cockpit dashboard with a new `CockpitPanel` border widget, focus-driven panel enlarging, a percussive-maintenance glitch mechanic, and a boot sequence. Sub-app content (Hyperdrive/Sensors/Weapons' actual functionality) is a follow-up plan; this one ships placeholders for each.

**Architecture:** One new core widget (`src/widgets/cockpit_panel.rs`) plus one small addition to the existing `GlitchBuffer` (`src/glitch.rs`). Everything else is a new example app (`examples/falcon/`) assembled entirely from existing primitives — `GlitchBuffer`, `Transition`, `ParticleSystem`, `Layout`/`Constraint`, `Theme`, `LayerStack` — following the exact structural conventions already established by `examples/smash_crabs/` and `examples/launcher/`.

**Tech Stack:** Rust, existing `ttui` core and widget set.

## Global Constraints

- Slice 1 (`CockpitPanel`) and the `GlitchBuffer::clear()` addition are **`coding`-tagged: full TDD applies, no exceptions.**
- Everything under `examples/falcon/` is **example code: TDD-exempt** per `.claude/rules/development-conventions.md`'s "Examples/demos" exception — correctness is checked by running the example, not asserting on it.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are hard gates on every task.
- No new dependency. No new core rendering primitive — every mechanic reuses what already exists in `src/`.
- Every real `Cell {...}` literal needs `alpha: 1.0` explicit (never left to `..Default::default()`) — the Arc C invariant applies to all new code in this plan too.
- One worktree for this whole plan, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- `coding`-tagged → **Gated** autonomy tier: ships as a PR to `main` with all four required checks green, squash-merged at the end.
- Spec being implemented: `docs/design/specs/falcon/2026-08-09-falcon-dashboard-hub-design.md`.

---

### Task 1: `CockpitPanel` widget (`src/widgets/cockpit_panel.rs`)

**Files:**
- Create: `src/widgets/cockpit_panel.rs`
- Modify: `src/widgets/mod.rs` (register the new module)

**Interfaces:**
- Consumes: `crate::buffer::{Buffer, Cell}`, `crate::layout::Rect`, `crate::theme::Theme` (existing, unchanged).
- Produces: `pub struct CockpitPanel { pub focused: bool }` with `pub fn new(focused: bool) -> Self` and `pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect` — later tasks call this exactly like `SmashBorder::new().render(area, theme, buf)`.

- [ ] **Step 1: Write the failing tests**

Create `src/widgets/cockpit_panel.rs` with just the test module first:

```rust
//! Thick, riveted, deliberately-asymmetric double-line border for a
//! jury-rigged cockpit-panel look.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::theme::Theme;
use crossterm::style::Color;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::BorderSet;

    fn test_theme() -> Theme {
        Theme {
            background: Color::Black,
            primary: Color::Rgb {
                r: 255,
                g: 176,
                b: 0,
            },
            secondary: Color::Rgb {
                r: 76,
                g: 187,
                b: 23,
            },
            tertiary: Color::Red,
            accent: Color::Yellow,
            primary_end: None,
            border: BorderSet::default(),
            border_bold: false,
            border_thick: false,
        }
    }

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        }
    }

    #[test]
    fn focused_panel_uses_theme_primary_for_both_rings() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(true).render(area(), &theme, &mut buf);
        assert_eq!(buf.get(0, 0).fg, theme.primary);
        assert_eq!(buf.get(1, 1).fg, theme.primary);
    }

    #[test]
    fn unfocused_panel_uses_theme_secondary_for_both_rings() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(false).render(area(), &theme, &mut buf);
        assert_eq!(buf.get(0, 0).fg, theme.secondary);
        assert_eq!(buf.get(1, 1).fg, theme.secondary);
    }

    #[test]
    fn three_corners_are_plus_and_bottom_right_is_the_one_asymmetric_glyph() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(true).render(area(), &theme, &mut buf);
        assert_eq!(buf.get(0, 0).symbol, '+');
        assert_eq!(buf.get(9, 0).symbol, '+');
        assert_eq!(buf.get(0, 7).symbol, '+');
        assert_eq!(buf.get(9, 7).symbol, '¤');
    }

    #[test]
    fn rivets_appear_at_the_expected_deterministic_offsets_on_the_outer_ring() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(true).render(area(), &theme, &mut buf);
        // top edge (width 10): offset % 3 == 1 -> offsets 1, 4, 7 are rivets.
        assert_eq!(buf.get(1, 0).symbol, 'o');
        assert_eq!(buf.get(4, 0).symbol, 'o');
        assert_eq!(buf.get(7, 0).symbol, 'o');
        assert_eq!(buf.get(2, 0).symbol, '=');
        assert_eq!(buf.get(3, 0).symbol, '=');
        // left edge: offset % 2 == 1 -> offset 1 is a rivet, offset 2 isn't.
        assert_eq!(buf.get(0, 1).symbol, 'o');
        assert_eq!(buf.get(0, 2).symbol, '#');
    }

    #[test]
    fn inner_ring_has_no_rivets_and_no_asymmetry() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(true).render(area(), &theme, &mut buf);
        // inner ring: area shrunk by 1 on each side -> x in 1..9, y in 1..7.
        assert_eq!(buf.get(1, 1).symbol, '+');
        assert_eq!(buf.get(8, 1).symbol, '+');
        assert_eq!(buf.get(1, 6).symbol, '+');
        assert_eq!(buf.get(8, 6).symbol, '+'); // not asymmetric, unlike the outer ring
        assert_eq!(buf.get(2, 1).symbol, '-');
        assert_eq!(buf.get(1, 2).symbol, '|');
    }

    #[test]
    fn returns_area_shrunk_by_two_on_each_side() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        let inner = CockpitPanel::new(true).render(area(), &theme, &mut buf);
        assert_eq!(
            inner,
            Rect {
                x: 2,
                y: 2,
                width: 6,
                height: 4
            }
        );
    }

    #[test]
    fn too_small_area_degrades_gracefully_without_panic() {
        let theme = test_theme();
        let mut buf = Buffer::new(3, 3);
        let small = Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        };
        let inner = CockpitPanel::new(true).render(small, &theme, &mut buf);
        assert_eq!(
            inner,
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0
            }
        );
        assert_eq!(*buf.get(1, 1), Cell::default());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::cockpit_panel`
Expected: FAIL to compile — `CockpitPanel` doesn't exist yet.

- [ ] **Step 3: Implement `CockpitPanel`**

Add above the test module (after the `use` statements):

```rust
/// A two-ring border: an outer riveted ring with one intentionally
/// mismatched corner, and a plain inner ring — colored by `focused`.
pub struct CockpitPanel {
    pub focused: bool,
}

impl CockpitPanel {
    /// Creates a `CockpitPanel`; `focused` selects `theme.primary`
    /// (bright) vs `theme.secondary` (dimmed) for both rings.
    pub fn new(focused: bool) -> Self {
        CockpitPanel { focused }
    }

    /// Draws the outer riveted ring and inner plain ring, returning
    /// the shrunk inner content area. Degrades to a zero-size `Rect`
    /// without panicking when `area` is too small for both rings.
    pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect {
        if area.width < 4 || area.height < 4 {
            return Rect {
                x: area.x,
                y: area.y,
                width: 0,
                height: 0,
            };
        }

        let color = if self.focused {
            theme.primary
        } else {
            theme.secondary
        };

        for x in area.x..area.x + area.width {
            let offset = x - area.x;
            let glyph = if offset % 3 == 1 { 'o' } else { '=' };
            set_cell(buf, x, area.y, glyph, color);
            set_cell(buf, x, area.y + area.height - 1, glyph, color);
        }
        for y in area.y..area.y + area.height {
            let offset = y - area.y;
            let glyph = if offset % 2 == 1 { 'o' } else { '#' };
            set_cell(buf, area.x, y, glyph, color);
            set_cell(buf, area.x + area.width - 1, y, glyph, color);
        }
        set_cell(buf, area.x, area.y, '+', color);
        set_cell(buf, area.x + area.width - 1, area.y, '+', color);
        set_cell(buf, area.x, area.y + area.height - 1, '+', color);
        set_cell(
            buf,
            area.x + area.width - 1,
            area.y + area.height - 1,
            '¤',
            color,
        );

        let inner_outer = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width - 2,
            height: area.height - 2,
        };
        for x in inner_outer.x..inner_outer.x + inner_outer.width {
            set_cell(buf, x, inner_outer.y, '-', color);
            set_cell(buf, x, inner_outer.y + inner_outer.height - 1, '-', color);
        }
        for y in inner_outer.y..inner_outer.y + inner_outer.height {
            set_cell(buf, inner_outer.x, y, '|', color);
            set_cell(buf, inner_outer.x + inner_outer.width - 1, y, '|', color);
        }
        set_cell(buf, inner_outer.x, inner_outer.y, '+', color);
        set_cell(
            buf,
            inner_outer.x + inner_outer.width - 1,
            inner_outer.y,
            '+',
            color,
        );
        set_cell(
            buf,
            inner_outer.x,
            inner_outer.y + inner_outer.height - 1,
            '+',
            color,
        );
        set_cell(
            buf,
            inner_outer.x + inner_outer.width - 1,
            inner_outer.y + inner_outer.height - 1,
            '+',
            color,
        );

        Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
        }
    }
}

fn set_cell(buf: &mut Buffer, x: u16, y: u16, symbol: char, color: Color) {
    buf.set(
        x,
        y,
        Cell {
            symbol,
            fg: color,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        },
    );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::cockpit_panel`
Expected: PASS — all 7 tests.

- [ ] **Step 5: Register the module**

In `src/widgets/mod.rs`, add in alphabetical order (after `block`, before `damage_meter`):

```rust
/// Thick, riveted, deliberately-asymmetric double-line border.
pub mod cockpit_panel;
```

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy --lib -- -D warnings` and `cargo fmt --check -- src/widgets/cockpit_panel.rs src/widgets/mod.rs`
Expected: both clean.

- [ ] **Step 7: Commit**

```bash
git add src/widgets/cockpit_panel.rs src/widgets/mod.rs
git commit -m "feat(falcon): add CockpitPanel widget

Thick, riveted, deliberately-asymmetric double-line border for the
Falcon dashboard hub — same custom-border-renderer family as
SmashBorder, one intentional mismatched corner as the 'bolted
together by hand' signature detail the vision doc calls for."
```

---

### Task 2: `GlitchBuffer::clear()` (`src/glitch.rs`)

**Files:**
- Modify: `src/glitch.rs`

**Interfaces:**
- Consumes: `GlitchBuffer`'s existing private `transition: Option<Transition>` field (same file, no cross-module change).
- Produces: `pub fn clear(&mut self)` — Task 4 calls this on the focused panel's `GlitchBuffer` when the player presses the WHACK key.

- [ ] **Step 1: Write the failing test**

Add to `src/glitch.rs`'s `#[cfg(test)] mod tests`, after `ticking_past_the_triggered_duration_deactivates_it`:

```rust
    #[test]
    fn clear_deactivates_immediately_regardless_of_remaining_duration() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(600));
        assert!(gb.is_active());
        gb.clear();
        assert!(!gb.is_active());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib glitch::tests::clear_deactivates_immediately_regardless_of_remaining_duration`
Expected: FAIL to compile — `clear` doesn't exist yet.

- [ ] **Step 3: Implement `clear()`**

In `src/glitch.rs`'s `impl GlitchBuffer` block, after `tick`:

```rust
    /// Ends the glitch immediately, regardless of remaining duration.
    pub fn clear(&mut self) {
        self.transition = None;
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib glitch::tests`
Expected: PASS — all tests in this module, including the new one.

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy --lib -- -D warnings` and `cargo fmt --check -- src/glitch.rs`
Expected: both clean.

- [ ] **Step 6: Commit**

```bash
git add src/glitch.rs
git commit -m "feat(falcon): add GlitchBuffer::clear() for early-dismissal

Falcon's percussive-maintenance mechanic needs to end a glitch on
demand (the player 'whacked' it) rather than only ever waiting out
its natural decay."
```

---

### Task 3: Falcon app skeleton (`examples/falcon/{main.rs,falcon.rs}`)

**Files:**
- Create: `examples/falcon/main.rs`
- Create: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: `ttui::app::App` (trait: `update(&mut self, event: &Event)`, `view(&self, area: Rect, buf: &mut LayerStack)`, `should_quit(&self) -> bool`, `tick_rate(&self) -> Option<Duration>`, `on_tick(&mut self, elapsed: Duration)` — exact signatures from `src/app.rs`), `ttui::widgets::cockpit_panel::CockpitPanel` (Task 1), `ttui::layout::{Layout, Direction, Constraint, Rect}`, `ttui::theme::{Theme, BorderSet}`, `ttui::widgets::text::Text`.
- Produces: `pub(crate) struct Falcon` and `pub(crate) fn new() -> Self` — Task 4 and Task 5 add fields/methods to this same struct and file.

Example code (TDD-exempt) — verified by running per this task's Step 4.

- [ ] **Step 1: Create the standalone entry point**

`examples/falcon/main.rs`:

```rust
// examples/falcon/main.rs — thin standalone entry; the App lives in
// falcon.rs so the launcher example can reuse it via #[path], same
// convention as every other themed app.
#[path = "falcon.rs"]
mod app;

fn main() -> std::io::Result<()> {
    ttui::app::run(&mut app::Falcon::new())
}
```

- [ ] **Step 2: Write the app skeleton**

`examples/falcon/falcon.rs`:

```rust
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::App;
use ttui::buffer::{Buffer, Cell, LayerStack};
use ttui::glitch::GlitchBuffer;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::particles::ParticleSystem;
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;
use ttui::widgets::{cockpit_panel::CockpitPanel, text::Text};

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS, matches every other app
const BOOT_TOTAL_MS: u64 = 1400;

#[derive(Clone, Copy, PartialEq)]
enum PanelKind {
    Hyperdrive,
    Sensors,
    Weapons,
}

const PANELS: [PanelKind; 3] = [
    PanelKind::Hyperdrive,
    PanelKind::Sensors,
    PanelKind::Weapons,
];

impl PanelKind {
    fn name(&self) -> &'static str {
        match self {
            PanelKind::Hyperdrive => "Hyperdrive",
            PanelKind::Sensors => "Sensors",
            PanelKind::Weapons => "Weapons",
        }
    }
}

fn falcon_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 10, g: 10, b: 8 },
        primary: Color::Rgb {
            r: 255,
            g: 176,
            b: 0,
        },
        secondary: Color::Rgb {
            r: 76,
            g: 187,
            b: 23,
        },
        tertiary: Color::Rgb {
            r: 255,
            g: 49,
            b: 49,
        },
        accent: Color::Rgb {
            r: 255,
            g: 215,
            b: 0,
        },
        primary_end: None,
        border: BorderSet::default(),
        border_bold: false,
        border_thick: false,
    }
}

pub(crate) struct Falcon {
    theme: Theme,
    focused: usize,
    // Task 4 adds `last_area`/`glitches`/`particles`/`tick_count` here.
    // Task 5 adds `booting` here.
    quit: bool,
}

impl Falcon {
    pub(crate) fn new() -> Self {
        Falcon {
            theme: falcon_theme(),
            focused: 0,
            quit: false,
        }
    }

    fn panel_slots(area: Rect) -> [Rect; 3] {
        let slots = Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 3]).split(area);
        [slots[0], slots[1], slots[2]]
    }

    fn panel_box(slot: Rect, focused: bool) -> Rect {
        let base_w = slot.width.saturating_sub(2).max(8);
        let base_h = slot.height.saturating_sub(4).clamp(4, 10);
        let focus_w = (base_w + 4).min(slot.width.saturating_sub(1));
        let focus_h = (base_h + 2).min(slot.height.saturating_sub(1));
        let box_w = if focused { focus_w } else { base_w };
        let box_h = if focused { focus_h } else { base_h };
        Rect {
            x: slot.x + slot.width.saturating_sub(box_w) / 2,
            y: slot.y + slot.height.saturating_sub(box_h) / 2,
            width: box_w,
            height: box_h,
        }
    }

    fn render_dashboard(&self, area: Rect, buf: &mut LayerStack) {
        let bg = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            alpha: 1.0,
            ..Default::default()
        };
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(area.x + x, area.y + y, bg.clone());
            }
        }

        let slots = Self::panel_slots(area);
        for (i, kind) in PANELS.iter().enumerate() {
            let focused = i == self.focused;
            let panel_box = Self::panel_box(slots[i], focused);
            let inner = CockpitPanel::new(focused).render(panel_box, &self.theme, buf);
            Text::new(kind.name()).render(inner, buf);
            if inner.height > 1 {
                let hint = Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: 1,
                };
                Text::new("(not yet built)").render(hint, buf);
            }
        }
    }
}

impl App for Falcon {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        match k.code {
            KeyCode::Tab => self.focused = (self.focused + 1) % PANELS.len(),
            KeyCode::BackTab => self.focused = (self.focused + PANELS.len() - 1) % PANELS.len(),
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.render_dashboard(area, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }
}
```

- [ ] **Step 3: Run clippy and fmt**

Run: `cargo clippy --example falcon -- -D warnings` and `cargo fmt --check -- examples/falcon/main.rs examples/falcon/falcon.rs`
Expected: both clean.

- [ ] **Step 4: Run the example to verify it builds and starts**

Run: `cargo build --example falcon`
Expected: builds successfully. If you have a way to run it interactively in this environment, run `cargo run --example falcon` and confirm: three bordered panels appear side by side, each showing its name and "(not yet built)"; the leftmost (Hyperdrive) is enlarged/brighter by default; Tab moves the enlarged/bright panel rightward, Shift+Tab moves it leftward, both wrapping; `q` quits cleanly. If no interactive terminal is available in this environment, confirm the build succeeds and state plainly that the interactive check could not be performed here — do not claim it was verified live if it wasn't.

- [ ] **Step 5: Commit**

```bash
git add examples/falcon/main.rs examples/falcon/falcon.rs
git commit -m "feat(falcon): add Falcon app skeleton with 3-panel dashboard

New fourth themed example: a smuggler-cockpit dashboard. Tab/Shift+Tab
cycle which of the three placeholder panels (Hyperdrive/Sensors/
Weapons) is focused, enlarged, and brightened; the other two sit
dimmed. Sub-app content is a follow-up plan — this ships placeholders."
```

---

### Task 4: Percussive maintenance (`examples/falcon/falcon.rs`)

**Files:**
- Modify: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: `ttui::glitch::GlitchBuffer` (`new`/`trigger`/`tick`/`is_active`/`render`/`clear` — `clear` from Task 2), `ttui::particles::{Particle, ParticleSystem}` (`new`/`spawn`/`update`/`render`), `Falcon::panel_slots`/`panel_box` (Task 3, same file).
- Produces: `Falcon.glitches: [GlitchBuffer; 3]`, `Falcon.particles: ParticleSystem`, `Falcon.tick_count: u64` — Task 5's boot sequence reuses the glitch-burst rendering this task establishes.

Example code (TDD-exempt) — verified by running per this task's Step 4.

- [ ] **Step 1: Add state**

In `examples/falcon/falcon.rs`, add to the top of the file:

```rust
use ttui::particles::Particle;
```

(alongside the existing `use ttui::particles::ParticleSystem;` — combine into one `use ttui::particles::{Particle, ParticleSystem};` line).

Add constants near the top, with the existing `TICK_INTERVAL`/`BOOT_TOTAL_MS`:

```rust
const IDLE_FLICKER_PERIOD_TICKS: u64 = 90; // ~3s at 33ms/tick, per panel
const IDLE_FLICKER_DURATION_MS: u64 = 600;
const WHACK_SPARK_COUNT: usize = 6;
const WHACK_SPARK_LIFETIME_MS: u64 = 300;
```

Update the `Falcon` struct (replacing the `// Task 4 adds...` comment):

```rust
pub(crate) struct Falcon {
    theme: Theme,
    focused: usize,
    // `App::view` takes `&self`, so this records the last-seen
    // terminal area through a `Cell` (interior mutability) rather
    // than a plain field, so `update`'s WHACK handler below can read
    // the focused panel's current on-screen position. Referenced by
    // full path (`std::cell::Cell`) rather than a `use` import,
    // since `ttui::buffer::Cell` is already imported under the plain
    // name `Cell` and the two would collide.
    last_area: std::cell::Cell<Rect>,
    glitches: [GlitchBuffer; 3],
    particles: ParticleSystem,
    tick_count: u64,
    // Task 5 adds `booting` here.
    quit: bool,
}
```

Update `Falcon::new()` (replacing the same comment):

```rust
    pub(crate) fn new() -> Self {
        Falcon {
            theme: falcon_theme(),
            focused: 0,
            last_area: std::cell::Cell::new(Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 24,
            }),
            glitches: [GlitchBuffer::new(), GlitchBuffer::new(), GlitchBuffer::new()],
            particles: ParticleSystem::new(),
            tick_count: 0,
            quit: false,
        }
    }
```

- [ ] **Step 2: Record `last_area` and wire the WHACK key**

Update `view` to record the area on every render (`std::cell::Cell::set` works through the shared `&self` reference):

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.last_area.set(area);
        self.render_dashboard(area, buf);
    }
```

Add the WHACK handler to `update`'s `match k.code` block (after the `KeyCode::BackTab` arm):

```rust
            KeyCode::Char(' ') => {
                if self.glitches[self.focused].is_active() {
                    self.glitches[self.focused].clear();
                    let slots = Self::panel_slots(self.last_area.get());
                    let panel_box = Self::panel_box(slots[self.focused], true);
                    let cx = panel_box.x as f32 + panel_box.width as f32 / 2.0;
                    let cy = panel_box.y as f32 + panel_box.height as f32 / 2.0;
                    for i in 0..WHACK_SPARK_COUNT {
                        let angle = i as f32 * std::f32::consts::TAU / WHACK_SPARK_COUNT as f32;
                        self.particles.spawn(Particle {
                            x: cx,
                            y: cy,
                            vx: angle.cos() * 6.0,
                            vy: angle.sin() * 3.0,
                            symbol: '*',
                            color: self.theme.accent,
                            lifetime: Duration::from_millis(WHACK_SPARK_LIFETIME_MS),
                            age: Duration::ZERO,
                        });
                    }
                }
            }
```

- [ ] **Step 3: Add `on_tick`**

Add to `impl App for Falcon` (after `tick_rate`):

```rust
    fn on_tick(&mut self, elapsed: Duration) {
        self.tick_count += 1;
        for (i, gb) in self.glitches.iter_mut().enumerate() {
            gb.tick(elapsed);
            if !gb.is_active()
                && self.tick_count % IDLE_FLICKER_PERIOD_TICKS == i as u64 * 30
            {
                gb.trigger(Duration::from_millis(IDLE_FLICKER_DURATION_MS));
            }
        }
        self.particles.update(elapsed);
    }
```

- [ ] **Step 4: Render the glitch overlay and sparks**

Change `render_dashboard` to composite the glitch overlay over each panel via a second `LayerStack` layer, and render particles on top:

```rust
    fn render_dashboard(&self, area: Rect, buf: &mut LayerStack) {
        let bg = Cell {
            symbol: ' ',
            fg: self.theme.primary,
            bg: self.theme.background,
            alpha: 1.0,
            ..Default::default()
        };
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(area.x + x, area.y + y, bg.clone());
            }
        }

        let slots = Self::panel_slots(area);
        let mut panel_inners = [Rect { x: 0, y: 0, width: 0, height: 0 }; 3];
        for (i, kind) in PANELS.iter().enumerate() {
            let focused = i == self.focused;
            let panel_box = Self::panel_box(slots[i], focused);
            let inner = CockpitPanel::new(focused).render(panel_box, &self.theme, buf);
            panel_inners[i] = inner;
            Text::new(kind.name()).render(inner, buf);
            if inner.height > 1 {
                let hint = Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: 1,
                };
                Text::new("(not yet built)").render(hint, buf);
            }
        }

        buf.push_layer();
        for (i, gb) in self.glitches.iter().enumerate() {
            if gb.is_active() {
                gb.render(panel_inners[i], self.theme.tertiary, self.tick_count, buf);
            }
        }
        self.particles.render(buf);
    }
```

`GlitchBuffer::render`'s signature is `render(&self, area: Rect, color: Color, tick_count: u64, buf: &mut Buffer)` (existing, `src/glitch.rs`) and `ParticleSystem::render`'s is `render(&self, buf: &mut Buffer)` (existing, `src/particles.rs`) — both called here with `buf: &mut LayerStack`, relying on `LayerStack`'s existing `DerefMut<Target = Buffer>` (same pattern already used throughout `examples/smash_crabs/`).

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy --example falcon -- -D warnings` and `cargo fmt --check -- examples/falcon/falcon.rs`
Expected: both clean.

- [ ] **Step 6: Run the example to verify it builds and starts**

Run: `cargo build --example falcon`
Expected: builds successfully. If interactive verification is possible in this environment: confirm idle (non-focused) panels occasionally show static/glitch overlay on their own and it clears without input after roughly half a second; confirm pressing Space while the focused panel is glitching clears it immediately with a brief spark burst at its center; confirm Space does nothing when the focused panel isn't glitching. If not possible here, state that plainly rather than claiming it was checked.

- [ ] **Step 7: Commit**

```bash
git add examples/falcon/falcon.rs
git commit -m "feat(falcon): add percussive maintenance mechanic

Each panel's GlitchBuffer fires on a staggered per-panel schedule for
ambient idle flicker, decaying on its own; Space clears the focused
panel's active glitch early with a particle-spark 'thunk' — the
Panel-Cycle + Percussive Maintenance interaction the vision doc
describes as this app's signature gesture."
```

---

### Task 5: Boot sequence (`examples/falcon/boot.rs`)

**Files:**
- Create: `examples/falcon/boot.rs`
- Modify: `examples/falcon/falcon.rs` (wire `booting` state in, add the `#[path] mod boot;` declaration)

**Interfaces:**
- Consumes: `Falcon`'s existing fields/methods (Tasks 3-4, same crate), `ttui::transition::Transition`, `ttui::easing::lerp_color`.
- Produces: `Falcon::render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack)`, called from `view` while `booting.is_some()`.

Example code (TDD-exempt) — verified by running per this task's Step 5.

- [ ] **Step 1: Wire `booting` state into `Falcon`**

In `examples/falcon/falcon.rs`, replace the `// Task 5 adds...` comment in the struct with:

```rust
    booting: Option<Transition>,
```

In `new()`, replace nothing (the field must be initialized) — add after `tick_count: 0,`:

```rust
            booting: Some(Transition::start(Duration::from_millis(BOOT_TOTAL_MS))),
```

Update `update` to ignore Tab/Shift+Tab/Space while booting (insert right after the `q` check, before the `match k.code` block):

```rust
        if self.booting.is_some() {
            return;
        }
```

Update `on_tick` to tick and clear `booting` (insert at the top of the function body, before `self.tick_count += 1;`):

```rust
        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            if t.is_complete() {
                self.booting = None;
            }
        }
```

Update `view` to branch to the boot renderer:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.last_area.set(area);
        if let Some(t) = &self.booting {
            self.render_boot(area, t.progress(), buf);
            return;
        }
        self.render_dashboard(area, buf);
    }
```

Add `mod boot;` wiring: at the top of `examples/falcon/falcon.rs` (after the existing `use` statements), add:

```rust
#[path = "boot.rs"]
mod boot;
```

This declares `boot.rs` as a submodule of `falcon.rs` itself (not `main.rs`) — matching `examples/smash_crabs/smash_crabs.rs`'s exact convention (`#[path = "boot.rs"] mod boot;` declared inside `smash_crabs.rs`, not `main.rs`), which is what lets `boot.rs`'s `use super::*;` (Step 2) resolve to `falcon.rs`'s items. `main.rs` itself needs no change in this task.

- [ ] **Step 2: Write `boot.rs`**

`examples/falcon/boot.rs` (uses `use super::*;` to access `Falcon`, `Rect`, `Cell`, `LayerStack`, `Color`, etc. from `falcon.rs` — same convention as `examples/smash_crabs/boot.rs`):

```rust
use super::*;

impl Falcon {
    pub(crate) fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        if progress < 0.1 {
            let cx = area.x + area.width / 2;
            let cy = area.y + area.height / 2;
            buf.set(
                cx,
                cy,
                Cell {
                    symbol: '\u{2022}', // '•'
                    fg: self.theme.primary,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            return;
        }

        if progress < 0.7 {
            let wave = (progress - 0.1) / 0.6;
            let panels_shown = ((wave * 3.0) as usize).min(3);
            let slots = Self::panel_slots(area);
            let mut newest_inner = None;
            for (i, kind) in PANELS.iter().enumerate().take(panels_shown) {
                let panel_box = Self::panel_box(slots[i], false);
                let inner = CockpitPanel::new(false).render(panel_box, &self.theme, buf);
                Text::new(kind.name()).render(inner, buf);
                newest_inner = Some(inner);
            }
            // The most-recently-revealed panel gets a static burst: a
            // freshly-triggered GlitchBuffer rendered in the same frame
            // (never ticked) always renders at full intensity, so this
            // panel flashes static until the next one takes over as
            // "newest" — the "brief static burst" the design spec calls
            // for at each panel's reveal moment.
            if let Some(inner) = newest_inner {
                let mut burst = GlitchBuffer::new();
                burst.trigger(Duration::from_millis(300));
                buf.push_layer();
                burst.render(inner, self.theme.tertiary, self.tick_count, buf);
            }
            return;
        }

        let fade = ((progress - 0.7) / 0.3).clamp(0.0, 1.0);
        // Render into an isolated scratch LayerStack, not `buf` directly:
        // render_dashboard pushes its own glitch/particle layer, and if we
        // dimmed cells directly on `buf` afterward we'd only be rewriting
        // its base layer — the un-dimmed glitch/particle layer would still
        // be there for app.rs's own final composite() to blend back in on
        // top, undoing the fade. Compositing the scratch stack down to a
        // flat Buffer first, then writing dimmed cells onto `buf` (which
        // stays single-layer throughout this branch), avoids that leak.
        let mut scratch = LayerStack::new(area.width, area.height);
        self.render_dashboard(
            Rect {
                x: 0,
                y: 0,
                width: area.width,
                height: area.height,
            },
            &mut scratch,
        );
        let composited = scratch.composite();
        for y in 0..area.height {
            for x in 0..area.width {
                let real = composited.get(x, y);
                let dimmed = Cell {
                    symbol: real.symbol,
                    fg: ttui::easing::lerp_color(self.theme.background, real.fg, fade),
                    bg: ttui::easing::lerp_color(self.theme.background, real.bg, fade),
                    style: real.style,
                    alpha: 1.0,
                };
                buf.set(area.x + x, area.y + y, dimmed);
            }
        }
    }
}
```

**Rendering, by `Transition::progress()`:**
- `[0.0, 0.1)`: single amber pilot-light glyph, screen otherwise untouched (default/transparent, matches `Cell::default()`).
- `[0.1, 0.7)`: panels snap in one at a time (`panels_shown` grows 0→3), each unfocused; the most-recently-revealed panel flashes a static burst until the next one appears.
- `[0.7, 1.0]`: full dashboard renders normally into an isolated scratch `LayerStack` (via the same `render_dashboard` this task reuses, panel 0 focused per `Falcon::new()`'s default), composited down to a flat `Buffer`, then every cell's `fg`/`bg` is dimmed from `theme.background` toward its real color by `fade` and written onto `buf` — a whole-frame dim-to-bright ramp using the existing `easing::lerp_color`.

`scratch.composite()` (`LayerStack`, Arc C) reads back the scratch stack's fully-blended dashboard as a flat `Buffer` without mutating `scratch` itself — used here purely to avoid re-deriving the dashboard's per-cell colors by hand before dimming them onto `buf`.

- [ ] **Step 3: Run clippy and fmt**

Run: `cargo clippy --example falcon -- -D warnings` and `cargo fmt --check -- examples/falcon/boot.rs examples/falcon/falcon.rs`
Expected: both clean.

- [ ] **Step 4: Run the example to verify it builds**

Run: `cargo build --example falcon`
Expected: builds successfully.

- [ ] **Step 5: Run the example to verify the boot sequence**

If interactive verification is possible in this environment: run `cargo run --example falcon` and confirm the boot sequence plays once at startup exactly as described above (pilot light → panels snap in one at a time with a glitch burst each → whole frame brightens to full), then settles into the normal dashboard with Hyperdrive focused; confirm `q` quits cleanly even mid-boot. If not possible here, state that plainly — confirm at minimum that the binary builds and that `cargo test`/clippy/fmt are clean, and say the live boot-sequence check could not be performed in this environment.

- [ ] **Step 6: Commit**

```bash
git add examples/falcon/boot.rs examples/falcon/falcon.rs
git commit -m "feat(falcon): add boot sequence

Dead-black pilot light, panels snapping in rivet-by-rivet with a
glitch burst each, then a whole-frame dim-to-bright fade into the
normal dashboard — matches the vision doc's intro splash."
```

---

### Task 6: Final verification and `examples/README.md` entry

**Files:**
- Modify: `examples/README.md`

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: full suite green, including Task 1's `CockpitPanel` tests and Task 2's `GlitchBuffer::clear()` test.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds, including the new `falcon` example.

- [ ] **Step 4: Add the `examples/README.md` entry**

Add a new bullet, in the same style as the existing entries, after the `launcher` entry and before `demo`:

```markdown
- **`falcon`** — a scrappy smuggler-freighter cockpit with three
  instrument panels (Hyperdrive, Sensors, Weapons — placeholders for
  now, a follow-up plan fills them in) and a percussive-maintenance
  glitch mechanic. Built from `TTUI-Ideas/vision/UI/idea-4-Falcon.md`.
```

- [ ] **Step 5: Manual visual check**

Per this project's established limitation: this environment has no interactive PTY. State plainly in your final report that the live visual/interaction checks described in Tasks 3-5's Step 4/5/6 could not be performed here if that's the case, and that the safety net is `cargo test`/clippy/fmt/build all passing plus the code having been written to match the approved design spec's exact geometry and interaction contract.

- [ ] **Step 6: Commit**

```bash
git add examples/README.md
git commit -m "docs(falcon): add falcon to examples/README.md"
```

---

## Final verification (whole plan)

- [ ] `cargo test` — full suite green.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo build --all-targets` — library, examples, benches all compile.
- [ ] `CockpitPanel`'s 7 tests and `GlitchBuffer::clear()`'s 1 test all pass.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this plan's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree`.
