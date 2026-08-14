# Flagship Showcase Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `showcase` — a new top-level `[[bin]]` target hosted by a robot mascot (`GripperMascot`) that showcases five TTUI techniques (mouse, particles, camera+glitch, chord input, data-viz) as auto-playing vignettes selected from a tile menu.

**Architecture:** Single `ShowcaseApp` struct implementing `ttui::app::App`, following `examples/omnitrix`'s established single-struct-with-sub-screens pattern. `Screen` (`Menu` / `Vignette(VignetteId)`) drives dispatch; each vignette owns its own state struct, constructed on entry and dropped on exit, stored as an `Option<XState>` field on `ShowcaseApp`. No new `src/`-level code — every vignette composes existing toolkit pieces (`particles.rs`, `glitch.rs`, `perspective.rs`/`canvas.rs`, `input.rs`, `sparkline.rs`/`bar_chart.rs`).

**Tech Stack:** Rust, `crossterm::event`, the existing `ttui` crate's `App`/`buffer`/`layout`/`theme`/`widgets`/`particles`/`glitch`/`perspective`/`canvas`/`input`/`transition`/`easing` modules.

## Global Constraints

- **TDD exemption applies to every file under `showcase/`** — this is demo code verified by building, running, and `tools/visual-snapshot` review, not by assertion, per `development-conventions.md`'s "Examples/demos" exemption applied by spirit (confirmed with the user during brainstorming). No task in this plan follows red-green-refactor; every task's cycle is: write the code, build, capture, review, commit. No task in this plan introduces new `src/`-level logic, so the "stays TDD" carve-out in that exemption never actually triggers here.
- **`showcase/` is a new top-level directory** (sibling to `src/`, `examples/`, `tools/`), with its own Cargo `[[bin]]` target — run via `cargo run --bin showcase`, not `cargo run --example showcase`.
- **Not indexed anywhere** — `examples/README.md` is not touched, and `showcase` is never reachable from `examples/launcher`'s portal nexus. Both deliberate per the approved spec's "outside the examples/ catalog" framing.
- **`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` must stay clean after every task** — `showcase` is a `[[bin]]` target, so it's included in `--all-targets` automatically.
- **`tools/visual-snapshot` capture + `Read`-and-review is mandatory** for every task that changes what's on screen (Tasks 2-9), per `development-conventions.md`'s "Visual review" convention applied by spirit to `showcase/`'s `view()`/`on_tick()`.
- **The human-only real-TTY checklist** (Task 10) covers Assembly Line (mouse) and Override Sequence (chord input) specifically — the same pattern already used for `control_panel`/`falcon`.
- **Navigation:** `Esc` returns from any vignette to the menu early; every vignette also auto-completes back to the menu with no user action required (see each vignette's own completion condition below). `q` quits only at the menu.
- **Exact type/field names below are load-bearing** — later tasks import them by these exact names. `Screen`/`VignetteId` are defined in `showcase.rs` (Task 1); every vignette module's state struct name, and its `new()`/`on_tick()`/`is_complete()`/`render()` method names, are fixed by this plan so `showcase.rs`'s dispatch (also fixed by this plan) compiles against them without modification task-to-task.

---

### Task 1: `showcase` bin target skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `showcase/main.rs`
- Create: `showcase/showcase.rs`

**Interfaces:**
- Produces: `pub(crate) enum VignetteId { AssemblyLine, OverloadVent, DiagnosticScan, OverrideSequence, Telemetry }` (`#[derive(Clone, Copy, PartialEq)]`), `pub(crate) struct ShowcaseApp` implementing `ttui::app::App`, `impl ShowcaseApp { pub(crate) fn new() -> Self }`. Every later task adds a `#[path = "..."] mod ...;` declaration and a dispatch arm inside `showcase.rs` — this task's `showcase.rs` is the file every later task edits, not replaces.

- [ ] **Step 1: Add the `[[bin]]` target**

In `Cargo.toml`, after the existing `[[example]]` block, add:

```toml
[[bin]]
name = "showcase"
path = "showcase/main.rs"
```

- [ ] **Step 2: Write the thin entry point**

Create `showcase/main.rs`, matching `examples/falcon/main.rs`'s exact convention:

```rust
// showcase/main.rs — thin standalone entry; the App lives in
// showcase.rs, matching the convention every themed example app uses.
#[path = "showcase.rs"]
mod app;

fn main() -> std::io::Result<()> {
    ttui::app::run(&mut app::ShowcaseApp::new())
}
```

- [ ] **Step 3: Write the `showcase.rs` skeleton**

Create `showcase/showcase.rs`:

```rust
//! showcase — the flagship demo reel. A robot mascot hosts a tile
//! menu of 5 auto-playing vignettes, each showcasing one TTUI
//! technique (mouse, particles, camera+glitch, chord input, data-viz).

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::App;
use ttui::buffer::{CellStyle, LayerStack};
use ttui::layout::Rect;
use ttui::theme::{BorderSet, Theme};
use ttui::transition::Transition;

const BOOT_MS: u64 = 1200;
const TICK_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum VignetteId {
    AssemblyLine,
    OverloadVent,
    DiagnosticScan,
    OverrideSequence,
    Telemetry,
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Menu,
    Vignette(VignetteId),
}

fn showcase_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 8, g: 8, b: 10 },
        primary: Color::Rgb {
            r: 138,
            g: 143,
            b: 152,
        },
        secondary: Color::Rgb {
            r: 199,
            g: 203,
            b: 209,
        },
        tertiary: Color::Rgb { r: 255, g: 60, b: 60 },
        accent: Color::Rgb {
            r: 255,
            g: 140,
            b: 66,
        },
        primary_end: None,
        border: BorderSet::default(),
        border_style: CellStyle::default(),
        border_thick: false,
    }
}

const ZERO_RECT: Rect = Rect {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};

pub(crate) struct ShowcaseApp {
    theme: Theme,
    screen: Screen,
    booting: Option<Transition>,
    last_area: std::cell::Cell<Rect>,
    quit: bool,
}

impl ShowcaseApp {
    pub(crate) fn new() -> Self {
        ShowcaseApp {
            theme: showcase_theme(),
            screen: Screen::Menu,
            booting: Some(Transition::start(Duration::from_millis(BOOT_MS))),
            last_area: std::cell::Cell::new(ZERO_RECT),
            quit: false,
        }
    }
}

impl App for ShowcaseApp {
    fn update(&mut self, event: &Event) {
        if self.booting.is_some() {
            return;
        }
        if self.screen == Screen::Menu {
            if let Event::Key(k) = event {
                if k.kind == KeyEventKind::Press && k.code == KeyCode::Char('q') {
                    self.quit = true;
                }
            }
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.last_area.set(area);
        if let Some(t) = &self.booting {
            let _ = t.progress();
            return;
        }
        if self.screen == Screen::Menu {
            // Task 3 replaces this with the real tile menu.
            let _ = &self.theme;
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, elapsed: Duration) {
        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            if t.is_complete() {
                self.booting = None;
            }
        }
    }
}
```

This is intentionally minimal (a boot delay then a blank menu screen) — later tasks fill in real rendering. It exists so every later task lands on top of a working, running binary rather than everyone's first task also being "make it build."

- [ ] **Step 4: Build and verify it runs**

Run: `cargo build --bin showcase`
Expected: succeeds, no warnings.

Run: `cargo run --bin showcase`, wait ~1.5s, press `q`.
Expected: a blank screen (boot delay, then blank menu), `q` quits cleanly back to a normal shell prompt.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml showcase/main.rs showcase/showcase.rs
git commit -m "feat(showcase): add showcase bin target skeleton

New top-level [[bin]] target (cargo run --bin showcase), distinct from
examples/ — the flagship demo reel's entry point. Boot delay + blank
menu only; mascot/menu/vignettes land in later commits."
```

---

### Task 2: The mascot — `GripperMascot`

**Files:**
- Create: `showcase/mascot.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub(crate) const MASCOT_WIDTH: u16 = 12`, `pub(crate) const MASCOT_HEIGHT: u16 = 12`, `pub(crate) enum MascotPose { Idle, Reacting, Grabbing }` (`Clone, Copy, PartialEq`), `pub(crate) struct GripperMascot` with `pub(crate) fn new() -> Self`, `pub(crate) fn set_pose(&mut self, pose: MascotPose)`, `pub(crate) fn tick(&mut self, elapsed: Duration)`, `pub(crate) fn render(&self, area: Rect, buf: &mut Buffer)` — consumed by Task 3 (idle beside the menu) and Task 5 (grabbing pose inside Assembly Line).

- [ ] **Step 1: Write `mascot.rs`**

Create `showcase/mascot.rs`:

```rust
//! GripperMascot — a 12x12-cell robot rendered as solid-color `Cell`s
//! (bg-fill, the same technique `list.rs`/`block.rs` use for row
//! highlighting), not glyph line-art. Three discrete poses; no
//! tweening between them, matching how every other app in this
//! project holds discrete poses rather than interpolating.

use crossterm::style::Color;
use std::time::Duration;
use ttui::buffer::{Buffer, Cell};
use ttui::layout::Rect;

pub(crate) const MASCOT_WIDTH: u16 = 12;
pub(crate) const MASCOT_HEIGHT: u16 = 12;

const REACT_HOLD: Duration = Duration::from_millis(300);
const GRAB_HOLD: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MascotPose {
    Idle,
    Reacting,
    Grabbing,
}

fn palette(code: u8) -> Option<Color> {
    match code {
        1 => Some(Color::Rgb {
            r: 42,
            g: 42,
            b: 42,
        }),
        2 => Some(Color::Rgb {
            r: 138,
            g: 143,
            b: 152,
        }),
        3 => Some(Color::Rgb {
            r: 255,
            g: 140,
            b: 66,
        }),
        4 => Some(Color::Rgb {
            r: 95,
            g: 212,
            b: 255,
        }),
        6 => Some(Color::Rgb {
            r: 199,
            g: 203,
            b: 209,
        }),
        _ => None,
    }
}

#[rustfmt::skip]
const IDLE: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,4,4,4,4,4,4,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,3,3,0,3,3,0,0,0,0],
];

#[rustfmt::skip]
const REACTING: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,2,4,4,4,4,2,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,3,3,0,3,3,0,0,0,0],
];

#[rustfmt::skip]
const GRABBING: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,4,4,4,4,4,4,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,0,3,3,3,3,0,0,0,0],
];

pub(crate) struct GripperMascot {
    pose: MascotPose,
    hold: Duration,
}

impl GripperMascot {
    pub(crate) fn new() -> Self {
        GripperMascot {
            pose: MascotPose::Idle,
            hold: Duration::ZERO,
        }
    }

    /// Switches pose immediately. `Reacting`/`Grabbing` auto-settle
    /// back to `Idle` after their hold duration elapses via `tick`.
    pub(crate) fn set_pose(&mut self, pose: MascotPose) {
        self.pose = pose;
        self.hold = match pose {
            MascotPose::Idle => Duration::ZERO,
            MascotPose::Reacting => REACT_HOLD,
            MascotPose::Grabbing => GRAB_HOLD,
        };
    }

    pub(crate) fn tick(&mut self, elapsed: Duration) {
        if self.hold > Duration::ZERO {
            self.hold = self.hold.saturating_sub(elapsed);
            if self.hold == Duration::ZERO {
                self.pose = MascotPose::Idle;
            }
        }
    }

    /// Draws the current pose's grid, one solid-color `Cell` per
    /// filled pixel, at `area`'s top-left corner. Cells clipped by
    /// `area` (or a grid entry of `0`) are simply skipped.
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer) {
        let grid = match self.pose {
            MascotPose::Idle => &IDLE,
            MascotPose::Reacting => &REACTING,
            MascotPose::Grabbing => &GRABBING,
        };
        for (row, cells) in grid.iter().enumerate() {
            let y = area.y + row as u16;
            if y >= area.y + area.height {
                break;
            }
            for (col, &code) in cells.iter().enumerate() {
                let x = area.x + col as u16;
                if x >= area.x + area.width {
                    break;
                }
                if let Some(color) = palette(code) {
                    buf.set(
                        x,
                        y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: color,
                            alpha: 1.0,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_mascot_starts_idle() {
        let m = GripperMascot::new();
        assert!(m.pose == MascotPose::Idle);
    }

    #[test]
    fn reacting_settles_back_to_idle_after_its_hold_elapses() {
        let mut m = GripperMascot::new();
        m.set_pose(MascotPose::Reacting);
        m.tick(REACT_HOLD);
        assert!(m.pose == MascotPose::Idle);
    }

    #[test]
    fn reacting_stays_active_before_its_hold_elapses() {
        let mut m = GripperMascot::new();
        m.set_pose(MascotPose::Reacting);
        m.tick(REACT_HOLD - Duration::from_millis(1));
        assert!(m.pose == MascotPose::Reacting);
    }

    #[test]
    fn render_skips_transparent_cells() {
        let m = GripperMascot::new();
        let mut buf = Buffer::new(MASCOT_WIDTH, MASCOT_HEIGHT);
        m.render(
            Rect {
                x: 0,
                y: 0,
                width: MASCOT_WIDTH,
                height: MASCOT_HEIGHT,
            },
            &mut buf,
        );
        // Grid row 0, col 0 is a `0` (transparent) in every pose.
        assert_eq!(*buf.get(0, 0), Cell::default());
        // Grid row 2, col 2 is a `2` (body) in every pose.
        assert_ne!(*buf.get(2, 2), Cell::default());
    }
}
```

Note: this file lives at `showcase/mascot.rs`, declared into the crate
via `#[path = "mascot.rs"] mod mascot;` inside `showcase.rs` (Step 2
below). It imports via the `ttui::` crate path, not `crate::`, because
`showcase` is its own binary crate that *depends on* `ttui` rather
than a module inside it — the same reason `examples/*.rs` files import
via `ttui::` throughout (e.g. `examples/mission_control.rs`'s `use
ttui::widgets::{...}`), whereas `src/widgets/*.rs` files use `crate::`
since they live inside the `ttui` library crate itself.

- [ ] **Step 2: Wire the mascot into `showcase.rs`**

In `showcase/showcase.rs`, add near the top (after the existing `use` block):

```rust
#[path = "mascot.rs"]
mod mascot;

use mascot::{GripperMascot, MascotPose};
```

Add a `mascot: GripperMascot` field to `ShowcaseApp` and initialize it
in `new()` via `mascot: GripperMascot::new(),`.

In `on_tick`, after the `booting` block, add:

```rust
        self.mascot.tick(elapsed);
```

In `view`, replace the `if self.screen == Screen::Menu { ... }` block
with:

```rust
        if self.screen == Screen::Menu {
            let mascot_area = Rect {
                x: area.x + area.width.saturating_sub(mascot::MASCOT_WIDTH + 2),
                y: area.y + 1,
                width: mascot::MASCOT_WIDTH,
                height: mascot::MASCOT_HEIGHT,
            };
            self.mascot.render(mascot_area, buf);
        }
```

- [ ] **Step 3: Run the mascot's own tests**

Run: `cargo test --bin showcase`
Expected: all 4 new tests in `mascot.rs` pass.

- [ ] **Step 4: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean.

- [ ] **Step 5: Capture and verify visually**

```
cargo run -p visual-snapshot -- --bin showcase --size 100x30 --script <2-second-wait-script.json> --out /tmp/mascot.gif
```

Use a script that waits past the 1200ms boot (e.g.
`[{"wait_ms": 1500}]`). `Read` the result. Confirm the mascot's idle
pose (bolted gray body, cyan LED visor band, orange claw) renders in
the top-right area, matching the design spec's grid data exactly (no
stray colored cells, no missing rows).

- [ ] **Step 6: Commit**

```bash
git add showcase/mascot.rs showcase/showcase.rs
git commit -m "feat(showcase): add GripperMascot pixel-tile rendering

Renders as solid-color Cells (bg-fill), not glyph line-art — the same
technique list.rs/block.rs use for row highlighting. Three discrete
poses (idle/reacting/grabbing), no tweening between them."
```

---

### Task 3: The tile menu

**Files:**
- Create: `showcase/menu.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: `VignetteId` (Task 1), `GripperMascot`/`MascotPose` (Task 2).
- Produces: `pub(crate) const TILES: [(VignetteId, &str, &str); 5]` (id, title, hint), `pub(crate) fn render_menu(area: Rect, theme: &Theme, highlighted: usize, buf: &mut LayerStack) -> [Rect; 5]` — consumed by Tasks 5-9 indirectly (each vignette's `TILES` entry is how the menu identifies it) and directly by `showcase.rs`'s mouse-click hit-testing.

- [ ] **Step 1: Write `menu.rs`**

Create `showcase/menu.rs`:

```rust
//! The tile menu: one row of 5 tiles, one per vignette. Arrow keys
//! move the highlight (mascot reacts on change, wired in showcase.rs);
//! Enter or a direct click on a tile launches it. Hover alone never
//! launches.

use super::VignetteId;
use ttui::buffer::LayerStack;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::Theme;
use ttui::widgets::{block::Block, text::Text};

pub(crate) const TILES: [(VignetteId, &str, &str); 5] = [
    (VignetteId::AssemblyLine, "Assembly Line", "click"),
    (VignetteId::OverloadVent, "Overload Vent", "watch"),
    (VignetteId::DiagnosticScan, "Diagnostic Scan", "space to whack"),
    (VignetteId::OverrideSequence, "Override Sequence", "chord"),
    (VignetteId::Telemetry, "Telemetry", "watch"),
];

const ZERO_RECT: Rect = Rect {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};

/// Renders the 5-tile row, returning each tile's outer `Rect` (border
/// included) for the caller to hit-test clicks against.
pub(crate) fn render_menu(
    area: Rect,
    theme: &Theme,
    highlighted: usize,
    buf: &mut LayerStack,
) -> [Rect; 5] {
    let cols = Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); TILES.len()]).split(area);
    let mut areas = [ZERO_RECT; 5];
    for (i, col) in cols.iter().enumerate() {
        let (_, title, hint) = TILES[i];
        let inner = Block::new().title(title).theme(theme).render(*col, buf);
        areas[i] = *col;
        let label = if i == highlighted {
            format!("> {hint}")
        } else {
            hint.to_string()
        };
        Text::new(&label).render(inner, buf);
    }
    areas
}
```

- [ ] **Step 2: Wire the menu into `showcase.rs`**

`menu.rs`'s `use super::VignetteId;` (Step 1) resolves correctly once
`menu.rs` is declared as a submodule *inside* `showcase.rs` (not
`main.rs`) — `super::` refers to `menu.rs`'s immediate parent module,
i.e. whatever module `showcase.rs` itself defines, regardless of what
`main.rs` calls that module from the outside (`app`, per Task 1).

Add near the top, alongside the `mascot` module declaration:

```rust
#[path = "menu.rs"]
mod menu;
```

Add two fields to `ShowcaseApp`: `highlighted: usize` and
`tile_areas: std::cell::Cell<[Rect; 5]>`. Initialize in `new()`:
`highlighted: 0,` and `tile_areas: std::cell::Cell::new([ZERO_RECT; 5]),`.

Replace `update`'s `Screen::Menu` branch body with:

```rust
        if self.screen == Screen::Menu {
            if let Event::Key(k) = event {
                if k.kind != KeyEventKind::Press {
                    return;
                }
                match k.code {
                    KeyCode::Char('q') => self.quit = true,
                    KeyCode::Left => {
                        let prev = self.highlighted;
                        self.highlighted = (self.highlighted + menu::TILES.len() - 1) % menu::TILES.len();
                        if self.highlighted != prev {
                            self.mascot.set_pose(MascotPose::Reacting);
                        }
                    }
                    KeyCode::Right => {
                        let prev = self.highlighted;
                        self.highlighted = (self.highlighted + 1) % menu::TILES.len();
                        if self.highlighted != prev {
                            self.mascot.set_pose(MascotPose::Reacting);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
```

(Enter-to-launch and click-to-launch are wired in Task 5, once there's
a real vignette to launch into — wiring navigation now without a
launch target would be dead code the reviewer can't exercise.)

Replace `view`'s mascot-only block with:

```rust
        if self.screen == Screen::Menu {
            let menu_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width.saturating_sub(mascot::MASCOT_WIDTH + 4),
                height: area.height,
            };
            let tile_areas = menu::render_menu(menu_area, &self.theme, self.highlighted, buf);
            self.tile_areas.set(tile_areas);
            let mascot_area = Rect {
                x: area.x + area.width.saturating_sub(mascot::MASCOT_WIDTH + 2),
                y: area.y + 1,
                width: mascot::MASCOT_WIDTH,
                height: mascot::MASCOT_HEIGHT,
            };
            self.mascot.render(mascot_area, buf);
        }
```

- [ ] **Step 3: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean.

- [ ] **Step 4: Capture and verify visually — idle menu**

Capture a single frame past the boot delay (as in Task 2 Step 5).
`Read` it. Confirm all 5 tiles render with their titles ("Assembly
Line", "Overload Vent", "Diagnostic Scan", "Override Sequence",
"Telemetry") and the first tile's hint is prefixed with `> ` (since
`highlighted` starts at `0`), mascot beside them.

- [ ] **Step 5: Capture and verify visually — highlight moves**

Script: `[{"wait_ms": 1500}, {"key": "Right"}, {"wait_ms": 200}]`.
`Read` the resulting frames. Confirm the `> ` prefix moved from tile 1
to tile 2, and the mascot's visor band shows the `Reacting` pattern in
the frame immediately after the `Right` key (narrower band than idle,
per the pixel grids).

- [ ] **Step 6: Commit**

```bash
git add showcase/menu.rs showcase/showcase.rs
git commit -m "feat(showcase): add the 5-tile vignette menu

Arrow-key highlight navigation with a mascot reaction on change; click
hit-testing areas are captured but not yet wired to anything (no
vignette exists to launch into until Task 5)."
```

---

### Task 4: Boot sequence

**Files:**
- Create: `showcase/boot.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub(crate) fn render_boot(area: Rect, theme: &Theme, progress: f32, buf: &mut LayerStack)`.

- [ ] **Step 1: Write `boot.rs`**

Create `showcase/boot.rs`:

```rust
//! Startup materialization: a centered logo fades in from the theme's
//! background to its primary color over the boot `Transition`'s
//! progress — the same background-to-color fade shape `falcon`'s own
//! boot sequence uses, at showcase's own (shorter) duration.

use crossterm::style::Color;
use ttui::buffer::{Cell, LayerStack};
use ttui::easing;
use ttui::layout::Rect;
use ttui::theme::Theme;

const LOGO: &str = "GRIPPER SHOWCASE";

pub(crate) fn render_boot(area: Rect, theme: &Theme, progress: f32, buf: &mut LayerStack) {
    let color = easing::lerp_color(theme.background, theme.primary, progress.clamp(0.0, 1.0));
    let cx = area.x + area.width.saturating_sub(LOGO.chars().count() as u16) / 2;
    let cy = area.y + area.height / 2;
    for (i, ch) in LOGO.chars().enumerate() {
        let x = cx + i as u16;
        if x >= area.x + area.width {
            break;
        }
        buf.set(
            x,
            cy,
            Cell {
                symbol: ch,
                fg: color,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
    }
}
```

- [ ] **Step 2: Wire it into `showcase.rs`**

Add near the top:

```rust
#[path = "boot.rs"]
mod boot;
```

In `view`, replace:

```rust
        if let Some(t) = &self.booting {
            let _ = t.progress();
            return;
        }
```

with:

```rust
        if let Some(t) = &self.booting {
            boot::render_boot(area, &self.theme, t.progress(), buf);
            return;
        }
```

- [ ] **Step 3: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean.

- [ ] **Step 4: Capture and verify visually**

Script: `[{"wait_ms": 300}, {"wait_ms": 300}, {"wait_ms": 300}]` (3
frames across the 1200ms boot). `Read` them. Confirm "GRIPPER SHOWCASE"
is dim/near-background in the first frame and visibly brighter
(closer to `theme.primary`'s gray) by the third, then a later frame
past 1200ms total shows the menu instead.

- [ ] **Step 5: Commit**

```bash
git add showcase/boot.rs showcase/showcase.rs
git commit -m "feat(showcase): add boot materialization sequence

Centered logo fade-in, matching the visual identity every other themed
app's boot sequence already gives its startup."
```

---

### Task 5: Vignette 1 — Assembly Line (mouse)

**Files:**
- Create: `showcase/mouse_grab.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: `VignetteId::AssemblyLine`, `menu::TILES`, `GripperMascot::set_pose`/`MascotPose::Grabbing` (Task 2), `Rect::contains` (existing, `src/layout.rs`).
- Produces: `pub(crate) struct AssemblyLineState` with `pub(crate) fn new() -> Self`, `pub(crate) fn on_tick(&mut self, elapsed: Duration, area: Rect)`, `pub(crate) fn handle_click(&mut self, mx: u16, my: u16)`, `pub(crate) fn take_caught(&mut self) -> bool`, `pub(crate) fn is_complete(&self) -> bool`, `pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack)`. This task also establishes the `enter_vignette`/`exit_vignette` pattern every later vignette task (6-9) reuses verbatim, and wires the menu's Enter-key/click launch that Task 3 deferred.

- [ ] **Step 1: Write `mouse_grab.rs`**

Create `showcase/mouse_grab.rs`:

```rust
//! Assembly Line — crates scroll across a fixed row; clicking one
//! before it exits triggers the mascot's grabbing pose and a small
//! particle puff where it was caught. Reuses control_panel's click
//! hit-testing pattern (a std::cell::Cell<u16> caching the clickable
//! row's y-coordinate from the last render, read back on click).

use crossterm::style::Color;
use std::time::Duration;
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::Rect;
use ttui::particles::{Particle, ParticleSystem};
use ttui::theme::Theme;

const CRATE_COUNT: usize = 6;
const CRATE_SPEED: f32 = 10.0; // cells/second
const CRATE_WIDTH: u16 = 6;
const SPAWN_INTERVAL: Duration = Duration::from_millis(700);
const PUFF_LIFETIME_MS: u64 = 300;

struct CrateItem {
    x: f32,
    caught: bool,
    exited: bool,
}

pub(crate) struct AssemblyLineState {
    crates: Vec<CrateItem>,
    spawn_elapsed: Duration,
    spawned: usize,
    just_caught: bool,
    puff: ParticleSystem,
    row_y: std::cell::Cell<u16>,
}

impl AssemblyLineState {
    pub(crate) fn new() -> Self {
        AssemblyLineState {
            crates: Vec::new(),
            spawn_elapsed: Duration::ZERO,
            spawned: 0,
            just_caught: false,
            puff: ParticleSystem::new(),
            row_y: std::cell::Cell::new(0),
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration, area: Rect) {
        self.just_caught = false;
        self.puff.update(elapsed);
        self.spawn_elapsed += elapsed;
        if self.spawned < CRATE_COUNT && self.spawn_elapsed >= SPAWN_INTERVAL {
            self.spawn_elapsed = Duration::ZERO;
            self.spawned += 1;
            self.crates.push(CrateItem {
                x: area.x as f32,
                caught: false,
                exited: false,
            });
        }
        let right_edge = (area.x + area.width) as f32;
        for c in &mut self.crates {
            if c.caught {
                continue;
            }
            c.x += CRATE_SPEED * elapsed.as_secs_f32();
            if c.x > right_edge {
                c.exited = true;
            }
        }
    }

    /// Hit-tests a click against the row cached from the last
    /// `render` call — mirrors `control_panel`'s `button_area`
    /// pattern (a Cell populated at render time, read at click time).
    pub(crate) fn handle_click(&mut self, mx: u16, my: u16) {
        if my != self.row_y.get() {
            return;
        }
        for c in &mut self.crates {
            if c.caught || c.exited {
                continue;
            }
            let cx = c.x as u16;
            if mx >= cx && mx < cx.saturating_add(CRATE_WIDTH) {
                c.caught = true;
                self.just_caught = true;
                self.puff.spawn(Particle {
                    x: c.x + CRATE_WIDTH as f32 / 2.0,
                    y: my as f32,
                    vx: 0.0,
                    vy: -3.0,
                    symbol: '*',
                    color: Color::Rgb {
                        r: 255,
                        g: 140,
                        b: 66,
                    },
                    lifetime: Duration::from_millis(PUFF_LIFETIME_MS),
                    age: Duration::ZERO,
                });
                break;
            }
        }
    }

    /// One-shot: true exactly once, the first call after a crate was
    /// caught since the last call.
    pub(crate) fn take_caught(&mut self) -> bool {
        std::mem::take(&mut self.just_caught)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.spawned == CRATE_COUNT && self.crates.iter().all(|c| c.caught || c.exited)
    }

    pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack) {
        let row_y = area.y + area.height / 2;
        self.row_y.set(row_y);
        for c in &self.crates {
            if c.caught || c.exited {
                continue;
            }
            let x = c.x as u16;
            for dx in 0..CRATE_WIDTH {
                let cx = x + dx;
                if cx >= area.x && cx < area.x + area.width {
                    buf.set(
                        cx,
                        row_y,
                        Cell {
                            symbol: '#',
                            fg: theme.secondary,
                            bg: Color::Reset,
                            alpha: 1.0,
                            ..Default::default()
                        },
                    );
                }
            }
        }
        let overlay = buf.push_layer();
        self.puff.render(overlay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 10,
        }
    }

    #[test]
    fn a_crate_spawns_after_the_first_spawn_interval() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        assert_eq!(s.spawned, 1);
        assert_eq!(s.crates.len(), 1);
    }

    #[test]
    fn a_crate_past_the_right_edge_is_marked_exited() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        s.on_tick(Duration::from_secs(10), area()); // plenty of time to cross a 40-wide area
        assert!(s.crates[0].exited);
    }

    #[test]
    fn clicking_a_crate_at_the_cached_row_catches_it() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        let theme = Theme::default();
        let mut stack = LayerStack::new(40, 10);
        s.render(area(), &theme, &mut stack);
        let row_y = area().y + area().height / 2;
        s.handle_click(0, row_y); // crate spawned at x=0
        assert!(s.crates[0].caught);
        assert!(s.take_caught());
    }

    #[test]
    fn take_caught_is_one_shot() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        let theme = Theme::default();
        let mut stack = LayerStack::new(40, 10);
        s.render(area(), &theme, &mut stack);
        let row_y = area().y + area().height / 2;
        s.handle_click(0, row_y);
        assert!(s.take_caught());
        assert!(!s.take_caught());
    }

    #[test]
    fn clicking_off_row_does_not_catch() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        let theme = Theme::default();
        let mut stack = LayerStack::new(40, 10);
        s.render(area(), &theme, &mut stack);
        s.handle_click(0, 0); // wrong row
        assert!(!s.crates[0].caught);
    }

    #[test]
    fn is_complete_once_all_spawned_crates_are_caught_or_exited() {
        let mut s = AssemblyLineState::new();
        for _ in 0..CRATE_COUNT {
            s.on_tick(SPAWN_INTERVAL, area());
        }
        assert_eq!(s.spawned, CRATE_COUNT);
        s.on_tick(Duration::from_secs(10), area()); // everything exits
        assert!(s.is_complete());
    }

    #[test]
    fn not_complete_until_every_crate_has_spawned() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        assert!(!s.is_complete());
    }
}
```

Note this test module gives `showcase/mouse_grab.rs` real unit tests
despite the TDD exemption — the exemption means these weren't written
test-first, not that tests are forbidden; this state machine has clear
enough inputs/outputs to be worth asserting on directly, the same way
`GripperMascot` (Task 2) got a small test module. Purely visual
concerns (exact rendering) stay verified by `visual-snapshot` instead.

- [ ] **Step 2: Wire it into `showcase.rs`**

Add near the top:

```rust
#[path = "mouse_grab.rs"]
mod mouse_grab;

use mouse_grab::AssemblyLineState;
```

Add fields to `ShowcaseApp`: `assembly_line: Option<AssemblyLineState>,`
— initialize `None` in `new()`.

Add these two methods to `impl ShowcaseApp`:

```rust
    fn enter_vignette(&mut self, id: VignetteId) {
        match id {
            VignetteId::AssemblyLine => self.assembly_line = Some(AssemblyLineState::new()),
            _ => {}
        }
        self.screen = Screen::Vignette(id);
    }

    fn exit_vignette(&mut self) {
        self.assembly_line = None;
        self.screen = Screen::Menu;
    }
```

(The `_ => {}` arm and the fact `exit_vignette` only clears one field
are deliberate — Tasks 6-9 each add their own match arm and their own
field-clear line, never touching this task's.)

Replace `update`'s whole body with:

```rust
    fn update(&mut self, event: &Event) {
        if self.booting.is_some() {
            return;
        }
        let screen = self.screen;
        match screen {
            Screen::Menu => {
                if let Event::Key(k) = event {
                    if k.kind != KeyEventKind::Press {
                        return;
                    }
                    match k.code {
                        KeyCode::Char('q') => self.quit = true,
                        KeyCode::Left => {
                            let prev = self.highlighted;
                            self.highlighted =
                                (self.highlighted + menu::TILES.len() - 1) % menu::TILES.len();
                            if self.highlighted != prev {
                                self.mascot.set_pose(MascotPose::Reacting);
                            }
                        }
                        KeyCode::Right => {
                            let prev = self.highlighted;
                            self.highlighted = (self.highlighted + 1) % menu::TILES.len();
                            if self.highlighted != prev {
                                self.mascot.set_pose(MascotPose::Reacting);
                            }
                        }
                        KeyCode::Enter => {
                            let id = menu::TILES[self.highlighted].0;
                            self.enter_vignette(id);
                        }
                        _ => {}
                    }
                } else if let Event::Mouse(m) = event {
                    if m.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                        for (i, area) in self.tile_areas.get().iter().enumerate() {
                            if area.contains(m.column, m.row) {
                                let id = menu::TILES[i].0;
                                self.enter_vignette(id);
                                return;
                            }
                        }
                    }
                }
            }
            Screen::Vignette(id) => {
                if let Event::Key(k) = event {
                    if k.kind == KeyEventKind::Press && k.code == KeyCode::Esc {
                        self.exit_vignette();
                        return;
                    }
                }
                if id == VignetteId::AssemblyLine {
                    if let (Some(state), Event::Mouse(m)) = (&mut self.assembly_line, event) {
                        if m.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                            state.handle_click(m.column, m.row);
                        }
                    }
                }
            }
        }
    }
```

Replace `on_tick`'s body with:

```rust
    fn on_tick(&mut self, elapsed: Duration) {
        if let Some(t) = &mut self.booting {
            t.tick(elapsed);
            if t.is_complete() {
                self.booting = None;
            }
            return;
        }
        self.mascot.tick(elapsed);
        let area = self.last_area.get();
        let screen = self.screen;
        match screen {
            Screen::Menu => {}
            Screen::Vignette(VignetteId::AssemblyLine) => {
                if let Some(state) = &mut self.assembly_line {
                    state.on_tick(elapsed, area);
                    let caught = state.take_caught();
                    let done = state.is_complete();
                    if caught {
                        self.mascot.set_pose(MascotPose::Grabbing);
                    }
                    if done {
                        self.exit_vignette();
                    }
                }
            }
            Screen::Vignette(_) => {}
        }
    }
```

Replace `view`'s body with:

```rust
    fn view(&self, area: Rect, buf: &mut LayerStack) {
        self.last_area.set(area);
        if let Some(t) = &self.booting {
            boot::render_boot(area, &self.theme, t.progress(), buf);
            return;
        }
        let mascot_area = Rect {
            x: area.x + area.width.saturating_sub(mascot::MASCOT_WIDTH + 2),
            y: area.y + 1,
            width: mascot::MASCOT_WIDTH,
            height: mascot::MASCOT_HEIGHT,
        };
        match self.screen {
            Screen::Menu => {
                let menu_area = Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width.saturating_sub(mascot::MASCOT_WIDTH + 4),
                    height: area.height,
                };
                let tile_areas = menu::render_menu(menu_area, &self.theme, self.highlighted, buf);
                self.tile_areas.set(tile_areas);
                self.mascot.render(mascot_area, buf);
            }
            Screen::Vignette(VignetteId::AssemblyLine) => {
                if let Some(state) = &self.assembly_line {
                    state.render(area, &self.theme, buf);
                }
                self.mascot.render(mascot_area, buf);
            }
            Screen::Vignette(_) => {}
        }
    }
```

(`Screen::Vignette(_) => {}` catch-alls in `on_tick`/`view` are
deliberate placeholders Tasks 6-9 each replace their own slice of —
every task after this one changes exactly one `Screen::Vignette(_) =>
{}` arm into a real one and leaves the others as `Screen::Vignette(_)
=> {}` until their own task lands.)

- [ ] **Step 3: Run tests**

Run: `cargo test --bin showcase`
Expected: all of Task 2's and this task's tests pass (10 total).

- [ ] **Step 4: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean. (`match screen { ... Screen::Vignette(_) => {} }`
alongside an earlier specific `Screen::Vignette(VignetteId::AssemblyLine)`
arm is exhaustive and idiomatic — clippy should not flag it, but if it
flags the match as better written as an `if let`, follow clippy's
suggestion since later tasks will re-expand it back to a full `match`
anyway.)

- [ ] **Step 5: Capture and verify visually — launching and catching**

Script:
```json
[{"wait_ms": 1500}, {"key": "Enter"}, {"wait_ms": 750}, {"x": 3, "y": 15}, {"wait_ms": 300}]
```
(adjust the click `y` to whatever row `render` actually places crates
at, for the `100x30` capture size — determine this the same way
`control_panel`'s plan Task 4 Step 5 did, by reasoning from the known
layout or inspecting an intermediate frame first). `Read` the result.
Confirm: Assembly Line launches on Enter, a crate is visible mid-row,
and after the click frame the mascot shows its `Grabbing` pose (closed
claw) with a small particle puff near the catch point.

- [ ] **Step 6: Capture and verify visually — Esc returns to menu**

Script: `[{"wait_ms": 1500}, {"key": "Enter"}, {"wait_ms": 300}, {"key": "Esc"}, {"wait_ms": 200}]`.
`Read` the result. Confirm the final frame shows the menu again, not
the vignette.

- [ ] **Step 7: Commit**

```bash
git add showcase/mouse_grab.rs showcase/showcase.rs
git commit -m "feat(showcase): add Assembly Line (mouse) vignette

Establishes the enter_vignette/exit_vignette dispatch pattern every
later vignette reuses. Crates scroll across a fixed row; clicking one
before it exits triggers the mascot's grabbing pose plus a particle
puff. Reuses control_panel's click hit-testing pattern."
```

---

### Task 6: Vignette 2 — Overload Vent (particles)

**Files:**
- Create: `showcase/particle_vent.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: `VignetteId::OverloadVent`.
- Produces: `pub(crate) struct OverloadVentState` with `pub(crate) fn new() -> Self`, `pub(crate) fn on_tick(&mut self, elapsed: Duration, area: Rect)`, `pub(crate) fn is_complete(&self) -> bool`, `pub(crate) fn render(&self, buf: &mut LayerStack)`.

- [ ] **Step 1: Write `particle_vent.rs`**

Create `showcase/particle_vent.rs`:

```rust
//! Overload Vent — 3 simultaneous particle emitters vent for a fixed
//! duration, exercising particles.rs more fully than control_panel's
//! single-button burst does. No interaction required.

use crossterm::style::Color;
use std::time::Duration;
use ttui::buffer::LayerStack;
use ttui::layout::Rect;
use ttui::particles::{Particle, ParticleSystem};
use ttui::transition::Transition;

const VENT_DURATION: Duration = Duration::from_millis(3500);
const EMIT_INTERVAL: Duration = Duration::from_millis(80);
const SPARK_LIFETIME_MS: u64 = 350;
const EMITTER_OFFSETS: [(f32, f32); 3] = [(-4.0, -2.0), (0.0, -3.0), (4.0, -2.0)];

pub(crate) struct OverloadVentState {
    particles: ParticleSystem,
    emit_elapsed: Duration,
    transition: Transition,
}

impl OverloadVentState {
    pub(crate) fn new() -> Self {
        OverloadVentState {
            particles: ParticleSystem::new(),
            emit_elapsed: Duration::ZERO,
            transition: Transition::start(VENT_DURATION),
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration, area: Rect) {
        self.transition.tick(elapsed);
        self.particles.update(elapsed);
        if self.transition.is_complete() {
            return;
        }
        self.emit_elapsed += elapsed;
        if self.emit_elapsed < EMIT_INTERVAL {
            return;
        }
        self.emit_elapsed = Duration::ZERO;
        let cx = area.x as f32 + area.width as f32 / 2.0;
        let cy = area.y as f32 + area.height as f32 / 2.0;
        for (i, &(ox, oy)) in EMITTER_OFFSETS.iter().enumerate() {
            let angle = (i as f32 / EMITTER_OFFSETS.len() as f32) * std::f32::consts::TAU
                + self.transition.progress() * std::f32::consts::TAU * 4.0;
            self.particles.spawn(Particle {
                x: cx + ox,
                y: cy + oy,
                vx: angle.cos() * 6.0,
                vy: angle.sin() * 3.0 - 2.0,
                symbol: '*',
                color: Color::Rgb {
                    r: 255,
                    g: 180,
                    b: 60,
                },
                lifetime: Duration::from_millis(SPARK_LIFETIME_MS),
                age: Duration::ZERO,
            });
        }
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.transition.is_complete() && self.particles.is_empty()
    }

    pub(crate) fn render(&self, buf: &mut LayerStack) {
        let overlay = buf.push_layer();
        self.particles.render(overlay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 20,
        }
    }

    #[test]
    fn emits_particles_from_all_three_emitters_on_first_interval() {
        let mut s = OverloadVentState::new();
        s.on_tick(EMIT_INTERVAL, area());
        assert_eq!(s.particles.len(), EMITTER_OFFSETS.len());
    }

    #[test]
    fn stops_emitting_once_the_vent_duration_completes() {
        let mut s = OverloadVentState::new();
        s.on_tick(VENT_DURATION, area()); // completes the transition
        let count_at_completion = s.particles.len();
        s.on_tick(EMIT_INTERVAL, area()); // would emit 3 more if still active
        assert_eq!(
            s.particles.len().saturating_sub(count_at_completion),
            0
        );
    }

    #[test]
    fn is_complete_once_duration_elapses_and_particles_fade() {
        let mut s = OverloadVentState::new();
        s.on_tick(VENT_DURATION, area());
        s.on_tick(Duration::from_secs(2), area()); // long enough for sparks to expire
        assert!(s.is_complete());
    }

    #[test]
    fn not_complete_while_duration_is_still_running() {
        let mut s = OverloadVentState::new();
        s.on_tick(Duration::from_millis(100), area());
        assert!(!s.is_complete());
    }
}
```

- [ ] **Step 2: Wire it into `showcase.rs`**

Add near the top:

```rust
#[path = "particle_vent.rs"]
mod particle_vent;

use particle_vent::OverloadVentState;
```

Add field `overload_vent: Option<OverloadVentState>,` (init `None`).

In `enter_vignette`'s match, add:

```rust
            VignetteId::OverloadVent => self.overload_vent = Some(OverloadVentState::new()),
```

In `exit_vignette`, add:

```rust
        self.overload_vent = None;
```

In `on_tick`'s match, replace the first `Screen::Vignette(_) => {}` with:

```rust
            Screen::Vignette(VignetteId::OverloadVent) => {
                if let Some(state) = &mut self.overload_vent {
                    state.on_tick(elapsed, area);
                    if state.is_complete() {
                        self.exit_vignette();
                    }
                }
            }
            Screen::Vignette(_) => {}
```

In `view`'s match, replace the `Screen::Vignette(_) => {}` with:

```rust
            Screen::Vignette(VignetteId::OverloadVent) => {
                if let Some(state) = &self.overload_vent {
                    state.render(buf);
                }
            }
            Screen::Vignette(_) => {}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --bin showcase`
Expected: all previous tests plus this task's 4 new tests pass.

- [ ] **Step 4: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean.

- [ ] **Step 5: Capture and verify visually**

Script: `[{"wait_ms": 1500}, {"key": "Right"}, {"key": "Enter"}, {"wait_ms": 500}, {"wait_ms": 500}]`
(`Right` once moves the highlight from tile 0 to tile 1, "Overload
Vent"). `Read` the result. Confirm multiple simultaneous spark
particles are visible, spreading from 3 distinct source points, and
auto-returns to the menu in a later frame (add a longer trailing wait
if 3.5s hasn't elapsed within the script above — extend with more
`{"wait_ms": 1000}` steps as needed to actually observe the
auto-return).

- [ ] **Step 6: Commit**

```bash
git add showcase/particle_vent.rs showcase/showcase.rs
git commit -m "feat(showcase): add Overload Vent (particles) vignette

3 simultaneous emitters for a fixed 3.5s, auto-returns to the menu
when done. No interaction required."
```

---

### Task 7: Vignette 3 — Diagnostic Scan (camera + glitch)

**Files:**
- Create: `showcase/camera_glitch.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: `VignetteId::DiagnosticScan`.
- Produces: `pub(crate) struct DiagnosticScanState` with `pub(crate) fn new() -> Self`, `pub(crate) fn on_tick(&mut self, elapsed: Duration)`, `pub(crate) fn whack(&mut self)`, `pub(crate) fn is_complete(&self) -> bool`, `pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack)`.

- [ ] **Step 1: Write `camera_glitch.rs`**

Create `showcase/camera_glitch.rs`:

```rust
//! Diagnostic Scan — a rotating 3D wireframe box (the gripper's arm
//! schematic), auto-glitching twice via GlitchBuffer; Space "whacks"
//! it clear early, mirroring falcon's percussive-maintenance mechanic
//! (examples/falcon/falcon.rs's FalconAction::Whack handler).

use std::time::Duration;
use ttui::buffer::LayerStack;
use ttui::canvas::{Canvas, CanvasMode};
use ttui::glitch::GlitchBuffer;
use ttui::layout::Rect;
use ttui::perspective::{Camera, Line3, Point3, ProjectLineParams};
use ttui::theme::Theme;

const ROTATION_SPEED: f32 = 1.2; // radians/second
const GLITCH_TRIGGER_AT: [Duration; 2] = [Duration::from_millis(1500), Duration::from_millis(3500)];
const GLITCH_DURATION: Duration = Duration::from_millis(600);
const BASE_Z: f32 = 6.0;

const CUBE_VERTS: [(f32, f32, f32); 8] = [
    (-1.0, -1.0, -1.0),
    (1.0, -1.0, -1.0),
    (1.0, 1.0, -1.0),
    (-1.0, 1.0, -1.0),
    (-1.0, -1.0, 1.0),
    (1.0, -1.0, 1.0),
    (1.0, 1.0, 1.0),
    (-1.0, 1.0, 1.0),
];
const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

fn rotate_y(v: (f32, f32, f32), angle: f32) -> Point3 {
    let (x, y, z) = v;
    Point3 {
        x: x * angle.cos() + z * angle.sin(),
        y,
        z: -x * angle.sin() + z * angle.cos() + BASE_Z,
    }
}

pub(crate) struct DiagnosticScanState {
    angle: f32,
    elapsed_total: Duration,
    glitch: GlitchBuffer,
    fired: [bool; 2],
    tick_count: u64,
}

impl DiagnosticScanState {
    pub(crate) fn new() -> Self {
        DiagnosticScanState {
            angle: 0.0,
            elapsed_total: Duration::ZERO,
            glitch: GlitchBuffer::new(),
            fired: [false, false],
            tick_count: 0,
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration) {
        self.tick_count += 1;
        self.elapsed_total += elapsed;
        self.angle = (self.angle + ROTATION_SPEED * elapsed.as_secs_f32()) % std::f32::consts::TAU;
        self.glitch.tick(elapsed);
        for i in 0..GLITCH_TRIGGER_AT.len() {
            if !self.fired[i] && self.elapsed_total >= GLITCH_TRIGGER_AT[i] {
                self.fired[i] = true;
                self.glitch.trigger(GLITCH_DURATION);
            }
        }
    }

    /// Clears an active glitch early — the "percussive maintenance"
    /// mechanic, same shape as falcon's Whack handler.
    pub(crate) fn whack(&mut self) {
        if self.glitch.is_active() {
            self.glitch.clear();
        }
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.fired[1] && !self.glitch.is_active()
    }

    pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack) {
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let cam = Camera {
            near: 0.5,
            focal_length: 8.0,
        };
        let params = ProjectLineParams {
            center_x: area.width as f32 / 2.0,
            center_y: area.height as f32 / 2.0,
            screen_w: area.width as f32 - 1.0 / 2.0,
            screen_h: area.height as f32 - 1.0 / 4.0,
            subpixels_x: 2.0,
            subpixels_y: 4.0,
            min_scale: 0.0,
        };
        for &(a, b) in CUBE_EDGES.iter() {
            let start = rotate_y(CUBE_VERTS[a], self.angle);
            let end = rotate_y(CUBE_VERTS[b], self.angle);
            if let Some((x0, y0, x1, y1)) = cam.project_line(Line3 { start, end }, params) {
                canvas.line(x0, y0, x1, y1, theme.primary);
            }
        }
        canvas.blit(buf, area.x, area.y);
        if self.glitch.is_active() {
            let overlay = buf.push_layer();
            self.glitch.render(area, theme.tertiary, self.tick_count, overlay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_glitch_fires_at_its_trigger_time() {
        let mut s = DiagnosticScanState::new();
        s.on_tick(GLITCH_TRIGGER_AT[0]);
        assert!(s.glitch.is_active());
        assert!(s.fired[0]);
        assert!(!s.fired[1]);
    }

    #[test]
    fn whack_clears_an_active_glitch() {
        let mut s = DiagnosticScanState::new();
        s.on_tick(GLITCH_TRIGGER_AT[0]);
        assert!(s.glitch.is_active());
        s.whack();
        assert!(!s.glitch.is_active());
    }

    #[test]
    fn whack_on_an_inactive_glitch_is_a_no_op() {
        let mut s = DiagnosticScanState::new();
        s.whack(); // nothing active yet
        assert!(!s.glitch.is_active());
    }

    #[test]
    fn is_complete_only_after_the_second_glitch_clears() {
        let mut s = DiagnosticScanState::new();
        s.on_tick(GLITCH_TRIGGER_AT[0]);
        assert!(!s.is_complete());
        s.whack();
        assert!(!s.is_complete(), "first glitch cleared, second hasn't fired yet");
        s.on_tick(GLITCH_TRIGGER_AT[1] - GLITCH_TRIGGER_AT[0]);
        assert!(s.fired[1]);
        assert!(!s.is_complete(), "second glitch just fired, still active");
        s.whack();
        assert!(s.is_complete());
    }

    #[test]
    fn angle_advances_with_elapsed_time() {
        let mut s = DiagnosticScanState::new();
        s.on_tick(Duration::from_millis(500));
        assert!(s.angle > 0.0);
    }
}
```

- [ ] **Step 2: Wire it into `showcase.rs`**

Add near the top:

```rust
#[path = "camera_glitch.rs"]
mod camera_glitch;

use camera_glitch::DiagnosticScanState;
```

Add field `diagnostic_scan: Option<DiagnosticScanState>,` (init `None`).

In `enter_vignette`'s match, add:

```rust
            VignetteId::DiagnosticScan => self.diagnostic_scan = Some(DiagnosticScanState::new()),
```

In `exit_vignette`, add:

```rust
        self.diagnostic_scan = None;
```

In `update`'s `Screen::Vignette(id)` branch, after the `AssemblyLine`
click-handling `if`, add:

```rust
                if id == VignetteId::DiagnosticScan {
                    if let (Some(state), Event::Key(k)) = (&mut self.diagnostic_scan, event) {
                        if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(' ') {
                            state.whack();
                        }
                    }
                }
```

In `on_tick`'s match, replace the current `Screen::Vignette(_) => {}`
with:

```rust
            Screen::Vignette(VignetteId::DiagnosticScan) => {
                if let Some(state) = &mut self.diagnostic_scan {
                    state.on_tick(elapsed);
                    if state.is_complete() {
                        self.exit_vignette();
                    }
                }
            }
            Screen::Vignette(_) => {}
```

In `view`'s match, replace the current `Screen::Vignette(_) => {}` with:

```rust
            Screen::Vignette(VignetteId::DiagnosticScan) => {
                if let Some(state) = &self.diagnostic_scan {
                    state.render(area, &self.theme, buf);
                }
            }
            Screen::Vignette(_) => {}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --bin showcase`
Expected: all previous tests plus this task's 5 new tests pass.

- [ ] **Step 4: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean.

- [ ] **Step 5: Capture and verify visually — rotation and auto-glitch**

Script: `[{"wait_ms": 1500}, {"key": "Right"}, {"key": "Right"}, {"key": "Enter"}, {"wait_ms": 1600}, {"wait_ms": 300}]`
(2 `Right`s move the highlight to tile 2, "Diagnostic Scan"). `Read`
the result. Confirm a rotating wireframe box renders via Braille-mode
canvas glyphs, and the frame after the first glitch's trigger time
shows visible glitch noise overlaid.

- [ ] **Step 6: Capture and verify visually — whack clears it**

Extend the script with `{"key": " "}` (Space) right after the glitch
becomes visible, then a short wait. `Read` the result. Confirm the
glitch noise disappears in the frame immediately after Space.

- [ ] **Step 7: Commit**

```bash
git add showcase/camera_glitch.rs showcase/showcase.rs
git commit -m "feat(showcase): add Diagnostic Scan (camera+glitch) vignette

A rotating wireframe box via perspective::Camera + Canvas (Braille
mode), auto-glitching twice; Space whacks it clear early, mirroring
falcon's percussive-maintenance mechanic."
```

---

### Task 8: Vignette 4 — Override Sequence (chord input)

**Files:**
- Create: `showcase/chord_override.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: `VignetteId::OverrideSequence`, `GripperMascot::set_pose`/`MascotPose::Reacting` (Task 2).
- Produces: `pub(crate) struct OverrideSequenceState` with `pub(crate) fn new() -> Self`, `pub(crate) fn handle_key(&mut self, event: &Event)`, `pub(crate) fn on_tick(&mut self, elapsed: Duration)`, `pub(crate) fn take_reaction(&mut self) -> bool`, `pub(crate) fn is_complete(&self) -> bool`, `pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack)`.

- [ ] **Step 1: Write `chord_override.rs`**

Create `showcase/chord_override.rs`:

```rust
//! Override Sequence — enter Left, Right, Left, Right (deliberately
//! distinct from falcon's Up,Up,Down,Down chord) to unlock "Turbo
//! Grip": a GlitchBuffer::with_alpha power-up flash plus a triumphant
//! mascot reaction, then auto-returns to the menu.

use crossterm::event::{Event, KeyCode};
use crossterm::style::Color;
use std::time::Duration;
use ttui::buffer::{Cell, LayerStack};
use ttui::glitch::GlitchBuffer;
use ttui::input::{InputBinder, KeyPress};
use ttui::layout::Rect;
use ttui::theme::Theme;
use ttui::transition::Transition;

const CHORD_TIMEOUT: Duration = Duration::from_millis(1500);
const FLASH_DURATION: Duration = Duration::from_millis(500);
const POST_UNLOCK_HOLD: Duration = Duration::from_millis(1500);
const PROMPT: &str = "Enter: Left, Right, Left, Right";
const UNLOCKED_TEXT: &str = "TURBO GRIP ONLINE";

#[derive(Clone, Copy, PartialEq)]
enum OverrideAction {
    Unlock,
}

pub(crate) struct OverrideSequenceState {
    input: InputBinder<OverrideAction>,
    unlocked: bool,
    pending_reaction: bool,
    flash: GlitchBuffer,
    hold: Option<Transition>,
    tick_count: u64,
}

impl OverrideSequenceState {
    pub(crate) fn new() -> Self {
        let mut input = InputBinder::new(CHORD_TIMEOUT);
        input.bind(
            vec![
                KeyPress::plain(KeyCode::Left),
                KeyPress::plain(KeyCode::Right),
                KeyPress::plain(KeyCode::Left),
                KeyPress::plain(KeyCode::Right),
            ],
            OverrideAction::Unlock,
        );
        OverrideSequenceState {
            input,
            unlocked: false,
            pending_reaction: false,
            flash: GlitchBuffer::new().with_alpha(0.5),
            hold: None,
            tick_count: 0,
        }
    }

    pub(crate) fn handle_key(&mut self, event: &Event) {
        if self.unlocked {
            return;
        }
        if self.input.feed(event) == Some(OverrideAction::Unlock) {
            self.unlocked = true;
            self.pending_reaction = true;
            self.flash.trigger(FLASH_DURATION);
            self.hold = Some(Transition::start(POST_UNLOCK_HOLD));
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration) {
        self.tick_count += 1;
        self.input.expire(elapsed);
        self.flash.tick(elapsed);
        if let Some(t) = &mut self.hold {
            t.tick(elapsed);
        }
    }

    /// One-shot: true exactly once, right after the chord unlocks.
    pub(crate) fn take_reaction(&mut self) -> bool {
        std::mem::take(&mut self.pending_reaction)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.hold.as_ref().map(|t| t.is_complete()).unwrap_or(false)
    }

    pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack) {
        let text = if self.unlocked { UNLOCKED_TEXT } else { PROMPT };
        let color = if self.unlocked {
            theme.accent
        } else {
            theme.secondary
        };
        let cx = area.x + area.width.saturating_sub(text.chars().count() as u16) / 2;
        let cy = area.y + area.height / 2;
        for (i, ch) in text.chars().enumerate() {
            let x = cx + i as u16;
            if x >= area.x + area.width {
                break;
            }
            buf.set(
                x,
                cy,
                Cell {
                    symbol: ch,
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
        }
        if self.flash.is_active() {
            let overlay = buf.push_layer();
            self.flash.render(area, theme.accent, self.tick_count, overlay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn the_full_chord_unlocks() {
        let mut s = OverrideSequenceState::new();
        s.handle_key(&press(KeyCode::Left));
        s.handle_key(&press(KeyCode::Right));
        s.handle_key(&press(KeyCode::Left));
        assert!(!s.unlocked);
        s.handle_key(&press(KeyCode::Right));
        assert!(s.unlocked);
    }

    #[test]
    fn an_incomplete_chord_does_not_unlock() {
        let mut s = OverrideSequenceState::new();
        s.handle_key(&press(KeyCode::Left));
        s.handle_key(&press(KeyCode::Right));
        assert!(!s.unlocked);
    }

    #[test]
    fn unlocking_sets_a_one_shot_reaction_flag() {
        let mut s = OverrideSequenceState::new();
        for code in [KeyCode::Left, KeyCode::Right, KeyCode::Left, KeyCode::Right] {
            s.handle_key(&press(code));
        }
        assert!(s.take_reaction());
        assert!(!s.take_reaction());
    }

    #[test]
    fn is_complete_only_after_the_post_unlock_hold_elapses() {
        let mut s = OverrideSequenceState::new();
        for code in [KeyCode::Left, KeyCode::Right, KeyCode::Left, KeyCode::Right] {
            s.handle_key(&press(code));
        }
        assert!(!s.is_complete());
        s.on_tick(POST_UNLOCK_HOLD);
        assert!(s.is_complete());
    }

    #[test]
    fn never_unlocked_is_never_complete() {
        let s = OverrideSequenceState::new();
        assert!(!s.is_complete());
    }
}
```

- [ ] **Step 2: Wire it into `showcase.rs`**

Add near the top:

```rust
#[path = "chord_override.rs"]
mod chord_override;

use chord_override::OverrideSequenceState;
```

Add field `override_sequence: Option<OverrideSequenceState>,` (init `None`).

In `enter_vignette`'s match, add:

```rust
            VignetteId::OverrideSequence => self.override_sequence = Some(OverrideSequenceState::new()),
```

In `exit_vignette`, add:

```rust
        self.override_sequence = None;
```

In `update`'s `Screen::Vignette(id)` branch, after the `DiagnosticScan`
`if`, add:

```rust
                if id == VignetteId::OverrideSequence {
                    if let Some(state) = &mut self.override_sequence {
                        state.handle_key(event);
                    }
                }
```

In `on_tick`'s match, replace the current `Screen::Vignette(_) => {}`
with:

```rust
            Screen::Vignette(VignetteId::OverrideSequence) => {
                if let Some(state) = &mut self.override_sequence {
                    state.on_tick(elapsed);
                    if state.take_reaction() {
                        self.mascot.set_pose(MascotPose::Reacting);
                    }
                    if state.is_complete() {
                        self.exit_vignette();
                    }
                }
            }
            Screen::Vignette(_) => {}
```

In `view`'s match, replace the current `Screen::Vignette(_) => {}` with:

```rust
            Screen::Vignette(VignetteId::OverrideSequence) => {
                if let Some(state) = &self.override_sequence {
                    state.render(area, &self.theme, buf);
                }
                self.mascot.render(mascot_area, buf);
            }
            Screen::Vignette(_) => {}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --bin showcase`
Expected: all previous tests plus this task's 5 new tests pass.

- [ ] **Step 4: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean.

- [ ] **Step 5: Capture and verify visually — the chord unlocks it**

Script:
```json
[{"wait_ms": 1500}, {"key": "Right"}, {"key": "Right"}, {"key": "Right"}, {"key": "Enter"}, {"wait_ms": 200}, {"key": "Left"}, {"key": "Right"}, {"key": "Left"}, {"key": "Right"}, {"wait_ms": 300}]
```
(3 `Right`s move the highlight to tile 3, "Override Sequence"). `Read`
the result. Confirm the prompt text is visible before the chord, then
"TURBO GRIP ONLINE" plus glitch-flash noise after it, and the mascot
shows `Reacting` in the frame right after the chord completes.

- [ ] **Step 6: Commit**

```bash
git add showcase/chord_override.rs showcase/showcase.rs
git commit -m "feat(showcase): add Override Sequence (chord input) vignette

Left,Right,Left,Right unlocks a GlitchBuffer::with_alpha power-up
flash and a mascot reaction, then auto-returns after a 1.5s hold."
```

---

### Task 9: Vignette 5 — Telemetry (data-viz)

**Files:**
- Create: `showcase/telemetry.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: `VignetteId::Telemetry`.
- Produces: `pub(crate) struct TelemetryState` with `pub(crate) fn new() -> Self`, `pub(crate) fn on_tick(&mut self, elapsed: Duration)`, `pub(crate) fn is_complete(&self) -> bool`, `pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack)`.

- [ ] **Step 1: Write `telemetry.rs`**

Create `showcase/telemetry.rs`:

```rust
//! Telemetry — live sparklines (Grip Force, Servo Load) and a bar
//! chart (Power Draw, Efficiency), same deterministic-random-walk
//! shape as examples/mission_control.rs, for a fixed 5.5s.

use std::time::Duration;
use ttui::buffer::LayerStack;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::Theme;
use ttui::transition::Transition;
use ttui::widgets::{bar_chart::BarChart, block::Block, sparkline::Sparkline};

const TELEMETRY_DURATION: Duration = Duration::from_millis(5500);
const GRIP_STEP: f32 = 6.0;
const SERVO_STEP: f32 = 5.0;
const STAT_STEP: f32 = 3.0;

fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}

pub(crate) struct TelemetryState {
    grip_force: f32,
    grip_force_history: Vec<f32>,
    servo_load: f32,
    servo_load_history: Vec<f32>,
    power_draw: f32,
    efficiency: f32,
    tick_count: u64,
    transition: Transition,
}

impl TelemetryState {
    pub(crate) fn new() -> Self {
        TelemetryState {
            grip_force: 60.0,
            grip_force_history: vec![60.0],
            servo_load: 40.0,
            servo_load_history: vec![40.0],
            power_draw: 70.0,
            efficiency: 85.0,
            tick_count: 0,
            transition: Transition::start(TELEMETRY_DURATION),
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration) {
        self.transition.tick(elapsed);
        self.tick_count += 1;
        let base = self.tick_count as u32;
        self.grip_force = (self.grip_force + scatter(base, GRIP_STEP)).clamp(0.0, 100.0);
        self.grip_force_history.push(self.grip_force);
        self.servo_load =
            (self.servo_load + scatter(base.wrapping_add(1_000), SERVO_STEP)).clamp(0.0, 100.0);
        self.servo_load_history.push(self.servo_load);
        self.power_draw =
            (self.power_draw + scatter(base.wrapping_add(2_000), STAT_STEP)).clamp(0.0, 100.0);
        self.efficiency =
            (self.efficiency + scatter(base.wrapping_add(3_000), STAT_STEP)).clamp(0.0, 100.0);
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.transition.is_complete()
    }

    pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack) {
        let rows = Layout::new(Direction::Vertical, vec![Constraint::Fill(1); 2]).split(area);
        let grip_inner = Block::new()
            .title("Grip Force")
            .theme(theme)
            .render(rows[0], buf);
        Sparkline::new(&self.grip_force_history, theme.primary).render(grip_inner, buf);

        let cols = Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 2]).split(rows[1]);
        let servo_inner = Block::new()
            .title("Servo Load")
            .theme(theme)
            .render(cols[0], buf);
        Sparkline::new(&self.servo_load_history, theme.accent).render(servo_inner, buf);

        let stats = [("Power Draw", self.power_draw), ("Efficiency", self.efficiency)];
        let stats_inner = Block::new().title("Stats").theme(theme).render(cols[1], buf);
        BarChart::new(&stats, 100.0, theme.secondary).render(stats_inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_tick_appends_to_every_history() {
        let mut s = TelemetryState::new();
        s.on_tick(Duration::from_millis(33));
        assert_eq!(s.grip_force_history.len(), 2);
        assert_eq!(s.servo_load_history.len(), 2);
    }

    #[test]
    fn values_stay_within_the_0_to_100_clamp() {
        let mut s = TelemetryState::new();
        for _ in 0..500 {
            s.on_tick(Duration::from_millis(33));
        }
        assert!((0.0..=100.0).contains(&s.grip_force));
        assert!((0.0..=100.0).contains(&s.servo_load));
        assert!((0.0..=100.0).contains(&s.power_draw));
        assert!((0.0..=100.0).contains(&s.efficiency));
    }

    #[test]
    fn is_complete_only_after_the_fixed_duration_elapses() {
        let mut s = TelemetryState::new();
        s.on_tick(Duration::from_secs(1));
        assert!(!s.is_complete());
        s.on_tick(TELEMETRY_DURATION);
        assert!(s.is_complete());
    }
}
```

- [ ] **Step 2: Wire it into `showcase.rs`**

Add near the top:

```rust
#[path = "telemetry.rs"]
mod telemetry;

use telemetry::TelemetryState;
```

Add field `telemetry: Option<TelemetryState>,` (init `None`).

In `enter_vignette`'s match, add:

```rust
            VignetteId::Telemetry => self.telemetry = Some(TelemetryState::new()),
```

In `exit_vignette`, add:

```rust
        self.telemetry = None;
```

In `on_tick`'s match, replace the final `Screen::Vignette(_) => {}`
with:

```rust
            Screen::Vignette(VignetteId::Telemetry) => {
                if let Some(state) = &mut self.telemetry {
                    state.on_tick(elapsed);
                    if state.is_complete() {
                        self.exit_vignette();
                    }
                }
            }
```

(No more catch-all arm remains — every `VignetteId` variant now has
its own arm in `on_tick`'s `match screen`, matching `enter_vignette`.)

In `view`'s match, replace the final `Screen::Vignette(_) => {}` with:

```rust
            Screen::Vignette(VignetteId::Telemetry) => {
                if let Some(state) = &self.telemetry {
                    state.render(area, &self.theme, buf);
                }
            }
```

- [ ] **Step 3: Run tests**

Run: `cargo test --bin showcase`
Expected: all previous tests plus this task's 3 new tests pass.

- [ ] **Step 4: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean — this is also the first point every `match
screen` arm in `showcase.rs` is exhaustively spelled out per-vignette
rather than falling through a `Screen::Vignette(_) => {}` catch-all;
confirm the compiler accepts it with no remaining catch-all needed.

- [ ] **Step 5: Capture and verify visually**

Script:
```json
[{"wait_ms": 1500}, {"key": "Left"}, {"key": "Enter"}, {"wait_ms": 400}, {"wait_ms": 400}, {"wait_ms": 400}]
```
(one `Left` from tile 0 wraps to tile 4, "Telemetry"). `Read` the
result. Confirm the Grip Force / Servo Load sparklines and the Stats
bar chart all show visibly different values across frames (proving the
random walk actually advances, the same check `mission_control`'s own
verification used), and a later frame (extend the trailing waits past
5.5s total if needed) shows the menu again.

- [ ] **Step 6: Commit**

```bash
git add showcase/telemetry.rs showcase/showcase.rs
git commit -m "feat(showcase): add Telemetry (data-viz) vignette

Live sparklines + bar chart, same deterministic-random-walk shape as
mission_control, for a fixed 5.5s. Completes the tile menu — all 5
vignettes are now wired end-to-end."
```

---

### Task 10: Final verification and human-only checklist

**Files:** none (verification only).

- [ ] **Step 1: Full workspace build, lint, format, test**

Run: `cargo build --all-targets` — succeeds.
Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.
Run: `cargo test` — full workspace suite green, including every test
added across Tasks 2, 5, 6, 7, 8, 9 (4 + 6 + 4 + 5 + 5 + 3 = 27 new
tests) plus everything that existed before this plan.

- [ ] **Step 2: One full end-to-end visual-snapshot capture**

Script exercising all 5 vignettes in one run, each entered from the
menu, given enough time to auto-complete or be skipped via `Esc`
(adjust exact wait/skip timings based on what Tasks 5-9's own captures
already established works):

```json
[
  {"wait_ms": 1500},
  {"key": "Enter"}, {"wait_ms": 1000}, {"key": "Esc"}, {"wait_ms": 200},
  {"key": "Right"}, {"key": "Enter"}, {"wait_ms": 1000}, {"key": "Esc"}, {"wait_ms": 200},
  {"key": "Right"}, {"key": "Right"}, {"key": "Enter"}, {"wait_ms": 1000}, {"key": "Esc"}, {"wait_ms": 200},
  {"key": "Right"}, {"key": "Right"}, {"key": "Right"}, {"key": "Enter"},
  {"wait_ms": 200}, {"key": "Left"}, {"key": "Right"}, {"key": "Left"}, {"key": "Right"},
  {"wait_ms": 1000}, {"key": "Esc"}, {"wait_ms": 200},
  {"key": "Right"}, {"key": "Right"}, {"key": "Right"}, {"key": "Right"}, {"key": "Enter"}, {"wait_ms": 1000}
]
```

`Read` the resulting GIF (or its extracted frames if the Read tool
only surfaces frame 1 for multi-frame GIFs — per the Cross-Platform
Verification plan's Task 3 finding, use the same PIL-frame-extraction
workaround if that turns out to be needed here too). Confirm every
vignette's key visual signature is present somewhere in the sequence:
crates + grabbing pose, multi-emitter sparks, rotating wireframe +
glitch, the unlock flash, and the sparklines/bar chart. Reference this
capture in the PR's Verification section.

- [ ] **Step 3: Deliver the human-only checklist**

Present this exact checklist to the user (do not paraphrase or
shorten it):

1. Open a real terminal window — not through Claude Code.
2. `cd` to this worktree.
3. `cargo run --bin showcase`. Wait for the boot sequence, then use
   arrow keys to move the highlight across all 5 tiles and confirm the
   mascot's LED visor visibly reacts each time the highlight moves.
4. Press Enter on "Assembly Line". Click a scrolling crate before it
   exits and confirm the mascot's claw visibly closes and a small
   particle puff appears. Press `q` after — confirm nothing happens
   (quit only works at the menu) — then press `Esc` to return, then
   `q` to quit. Confirm the terminal returns to a normal shell prompt
   — cursor visible, no leftover alternate-screen artifacts. Type any
   command (e.g. `ls`) afterward and confirm it echoes and behaves
   normally.
5. `cargo run --bin showcase` again. Wait for the boot, arrow over to
   "Override Sequence", press Enter, then enter the chord Left, Right,
   Left, Right. Confirm "TURBO GRIP ONLINE" appears with a brief flash
   effect, then it auto-returns to the menu on its own after a moment.
   Press `q` to quit and confirm the terminal restores cleanly again,
   same check as step 4.
6. Report back here: did everything behave identically to what the
   automated captures showed? If anything looked wrong, stuck,
   garbled, or the terminal didn't restore cleanly, describe exactly
   what happened and which step it happened on.

- [ ] **Step 4: Wait for the user's report**

Do not mark this task complete until the user has actually responded
with their observations.

- [ ] **Step 5: Record the result and file any finding**

No additional commit for this step. Note the user's reported outcome.
If they report any problem, or if any automated step in this plan
surfaced a genuine gap, file it as a GitHub issue and triage it per
`code-forge.md`'s rule (referenced in this plan's Global Constraints).
If zero findings resulted from every task in this plan, that's a
valid, complete outcome — say so plainly rather than treating "nothing
to file" as unfinished work.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` / `cargo clippy --all-targets -- -D
      warnings` / `cargo fmt --check` — all clean.
- [ ] `cargo test` — full suite green, including all 27 new tests
      across Tasks 2 and 5-9.
- [ ] Every one of Tasks 2-9's `tools/visual-snapshot` captures was
      actually reviewed (`Read`), plus Task 10's full end-to-end
      capture.
- [ ] The human-only checklist (Task 10) was delivered verbatim and
      the user's report was actually received, not skipped.
- [ ] Every genuine finding (if any) is filed as a GitHub issue with
      the correct `semver:*`/`v1-blocking` labels — `showcase/` is not
      part of `ttui`'s public API surface, so `code-forge.md`'s SemVer
      policy doesn't apply to it (same as `examples/` and
      `tools/visual-snapshot`); findings here default to
      `semver:patch`, no `v1-blocking`, unless a finding turns out to
      point at a genuine `src/`-level bug.
- [ ] Per `.claude/rules/git-github-standards.md`: this Arc is
      `coding`-tagged (real `src/`-adjacent behavior, even though the
      code itself lives outside `src/`) — Gated tier. Open a PR from
      this worktree's branch to `main`, wait for all four required
      checks green, squash-merge, then remove the worktree via
      `ExitWorktree`.
