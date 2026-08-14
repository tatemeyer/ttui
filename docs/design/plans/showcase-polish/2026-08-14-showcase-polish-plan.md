# Showcase Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `showcase/`'s mascot real idle motion and a more noticeable eye/blink, and rework Assembly Line into a real "the mascot walks over and picks it up" interaction — resolving GitHub issue #128 as part of the redesign.

**Architecture:** Two independent, internally-driven timers added to `GripperMascot` (breathing, blinking), consulted only while idle. Assembly Line's crate becomes a real pixel-tile sprite with 2D `Rect::contains` hit-testing; the mascot gains vignette-local sliding position state (owned by `AssemblyLineState`, reusing `ttui::transition::Transition` for the slide), replacing the shared top-right position for this one vignette only.

**Tech Stack:** Rust, existing `ttui` crate modules (`transition`, `layout::Rect::contains`, `particles`) — no new dependencies, no new `src/`-level code.

## Global Constraints

- **TDD exemption applies to every file under `showcase/`**, same as the original Flagship Showcase Arc — demo code verified by building, running, and `tools/visual-snapshot` review, not by assertion. Both tasks below do include real unit tests for state-machine logic (mascot timer behavior, Assembly Line's targeting/slide/catch sequencing) since that logic is worth asserting on directly, matching the precedent both files already set.
- **`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` must stay clean** after every task.
- **`tools/visual-snapshot` capture + `Read`-and-review is mandatory** for both tasks, per `development-conventions.md`'s visual-review convention.
- **The human-only real-TTY checklist covers Assembly Line again** (Task 3) — it's the vignette that already needed two live-testing-driven fixes in the original Arc.
- **Exact palette codes, grid data, and constants below are load-bearing** — Task 1's new palette code `9` and grid data, and Task 2's crate sprite/constants, are copied verbatim from the approved design spec; don't re-derive them.
- **Issue #128 is resolved by Task 2's redesign**, not a separate narrower fix — close it in Task 2's commit.
- **Issue #130 (BorderSet distinct corners) is explicitly out of scope** for this plan — do not touch `src/theme.rs`'s `BorderSet` or `Block::render`'s corner-placement logic.

---

### Task 1: Mascot idle animation and eye redesign

**Files:**
- Modify: `showcase/mascot.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: no public interface changes — `GripperMascot::new`/`set_pose`/`tick`/`render` keep their exact existing signatures. `showcase.rs` needs zero changes for this task.

- [ ] **Step 1: Update the palette and every grid's eye treatment**

In `showcase/mascot.rs`, add a new palette entry to the `palette` function (alongside the existing `1`/`2`/`3`/`4`/`6` arms):

```rust
        9 => Some(Color::Rgb {
            r: 255,
            g: 255,
            b: 255,
        }),
```

Replace the `IDLE`, `REACTING`, and `GRABBING` grid constants' visor row (row index 3 in each) with the two-tone version — every other row in these three grids stays byte-for-byte identical to what's already there:

```rust
// IDLE row 3 becomes:
    [0,2,2,4,9,9,4,4,4,2,2,0],
// REACTING row 3 becomes:
    [0,2,2,2,4,9,9,4,2,2,2,0],
// GRABBING row 3 becomes:
    [0,2,2,4,9,9,4,4,4,2,2,0],
```

- [ ] **Step 2: Add the `IDLE_B` and `BLINK` grids**

Add two new `#[rustfmt::skip]` grid constants, placed after the existing `GRABBING` constant:

```rust
#[rustfmt::skip]
const IDLE_B: [[u8; 12]; 12] = [
    [0,0,0,0,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,1,0,1,0,1,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,4,9,9,4,4,4,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,3,3,0,3,3,0,0,0,0],
];

#[rustfmt::skip]
const BLINK: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,1,1,1,1,1,1,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,3,3,0,3,3,0,0,0,0],
];
```

- [ ] **Step 3: Add the breathing/blink timers**

Add two new `Duration` constants near the existing `REACT_HOLD`/`GRAB_HOLD`:

```rust
const BREATHE_INTERVAL: Duration = Duration::from_millis(2000);
const BLINK_INTERVAL: Duration = Duration::from_millis(3500);
const BLINK_DURATION: Duration = Duration::from_millis(150);
```

Add two fields to `GripperMascot` and initialize them in `new()`:

```rust
pub(crate) struct GripperMascot {
    pose: MascotPose,
    hold: Duration,
    breathe_elapsed: Duration,
    blink_elapsed: Duration,
}
```

```rust
    pub(crate) fn new() -> Self {
        GripperMascot {
            pose: MascotPose::Idle,
            hold: Duration::ZERO,
            breathe_elapsed: Duration::ZERO,
            blink_elapsed: Duration::ZERO,
        }
    }
```

Replace `tick`'s body:

```rust
    /// Advances the pose-hold countdown (unchanged) plus two
    /// independent idle-animation timers. Both timers keep
    /// accumulating regardless of the current pose — only `render`
    /// gates their effect to `MascotPose::Idle` — so returning to
    /// `Idle` mid-cycle never causes a stutter or a reset-to-zero jump.
    pub(crate) fn tick(&mut self, elapsed: Duration) {
        if self.hold > Duration::ZERO {
            self.hold = self.hold.saturating_sub(elapsed);
            if self.hold == Duration::ZERO {
                self.pose = MascotPose::Idle;
            }
        }
        self.breathe_elapsed += elapsed;
        while self.breathe_elapsed >= BREATHE_INTERVAL {
            self.breathe_elapsed -= BREATHE_INTERVAL;
        }
        self.blink_elapsed += elapsed;
        let blink_cycle = BLINK_INTERVAL + BLINK_DURATION;
        while self.blink_elapsed >= blink_cycle {
            self.blink_elapsed -= blink_cycle;
        }
    }

    /// Second half of the breathing cycle: antenna/head dip.
    fn is_breathing_b(&self) -> bool {
        self.breathe_elapsed >= BREATHE_INTERVAL / 2
    }

    /// Within the held portion of the blink cycle.
    fn is_blinking(&self) -> bool {
        self.blink_elapsed >= BLINK_INTERVAL
    }
```

- [ ] **Step 4: Wire the new frames into `render`**

Replace `render`'s grid-selection `match`:

```rust
        let grid = match self.pose {
            MascotPose::Idle => {
                if self.is_blinking() {
                    &BLINK
                } else if self.is_breathing_b() {
                    &IDLE_B
                } else {
                    &IDLE
                }
            }
            MascotPose::Reacting => &REACTING,
            MascotPose::Grabbing => &GRABBING,
        };
```

- [ ] **Step 5: Update and add tests**

`render_skips_transparent_cells`'s existing assertions (row 0 col 0 is transparent, row 2 col 2 is body) are unaffected by these changes — leave that test as-is. Add these tests to the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn breathing_toggles_to_b_after_half_the_interval() {
        let mut m = GripperMascot::new();
        m.tick(BREATHE_INTERVAL / 2);
        assert!(m.is_breathing_b());
    }

    #[test]
    fn breathing_stays_a_before_half_the_interval() {
        let mut m = GripperMascot::new();
        m.tick(BREATHE_INTERVAL / 2 - Duration::from_millis(1));
        assert!(!m.is_breathing_b());
    }

    #[test]
    fn breathing_wraps_back_to_a_after_a_full_interval() {
        let mut m = GripperMascot::new();
        m.tick(BREATHE_INTERVAL);
        assert!(!m.is_breathing_b());
    }

    #[test]
    fn blinking_starts_after_the_blink_interval() {
        let mut m = GripperMascot::new();
        m.tick(BLINK_INTERVAL);
        assert!(m.is_blinking());
    }

    #[test]
    fn blinking_stays_false_before_the_blink_interval() {
        let mut m = GripperMascot::new();
        m.tick(BLINK_INTERVAL - Duration::from_millis(1));
        assert!(!m.is_blinking());
    }

    #[test]
    fn blinking_ends_after_its_own_duration_and_wraps() {
        let mut m = GripperMascot::new();
        m.tick(BLINK_INTERVAL + BLINK_DURATION);
        assert!(!m.is_blinking());
    }

    #[test]
    fn idle_timers_keep_accumulating_while_reacting() {
        let mut m = GripperMascot::new();
        m.set_pose(MascotPose::Reacting);
        m.tick(BREATHE_INTERVAL / 2);
        // Pose is still Reacting (REACT_HOLD is 300ms, well under
        // BREATHE_INTERVAL/2's 1000ms), so breathing has no visible
        // effect yet, but the timer itself must have kept moving —
        // verified indirectly: once it settles back to Idle, the
        // breathing phase should already reflect the elapsed time.
        m.tick(REACT_HOLD); // settles back to Idle (300ms < what's left of the hold)
        assert!(m.is_breathing_b());
    }

    #[test]
    fn render_selects_blink_grid_during_the_blink_window() {
        let mut m = GripperMascot::new();
        m.tick(BLINK_INTERVAL);
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
        // BLINK's row 3 is all code-1 (trim) across cols 3-8 — no
        // code-4/9 (visor) cells should be present on that row.
        for x in 3..9 {
            assert_ne!(
                buf.get(x, 3).bg,
                Color::Rgb {
                    r: 95,
                    g: 212,
                    b: 255
                },
                "visor should be dark during a blink, col {x}"
            );
        }
    }
```

Note: `idle_timers_keep_accumulating_while_reacting`'s second assertion
relies on `REACT_HOLD` (300ms) being visible in this test module —
it already is, as an existing private `const` in the same file.

- [ ] **Step 6: Run tests**

Run: `cargo test --bin showcase`
Expected: all previous `mascot` tests plus 8 new ones pass.

- [ ] **Step 7: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean.

- [ ] **Step 8: Capture and verify visually**

Using `tools/visual-snapshot` (`--bin showcase` flag, already exists), capture a sequence long enough to observe both the breathing toggle (past 2000ms) and at least one blink (past 3500ms) while at the menu (mascot idles there). A script like:

```json
[{"wait_ms": 1500}, {"wait_ms": 1000}, {"wait_ms": 1000}, {"wait_ms": 1500}]
```

`Read` the resulting frames (extract with ffmpeg/PIL if `Read` only shows frame 1 — a known limitation from earlier work on this project). Confirm: the antenna position visibly differs between an early and a later frame (breathing), and at least one frame shows the visor fully dark (blink) somewhere in the sequence — adjust wait durations and re-capture if the specific timing doesn't land a blink frame in the script as written, since blink timing relative to capture-tool quiescence-wait overhead (a known wrinkle from earlier work) may need a longer trailing wait to reliably observe.

- [ ] **Step 9: Commit**

```bash
git add showcase/mascot.rs
git commit -m "feat(showcase): add mascot idle animation and eye redesign

Two independent internally-driven timers (breathing, blinking),
consulted only while Idle — callers (showcase.rs) are unaffected,
still just calling set_pose/tick. New two-tone eye (white pupil core)
applied to every open-eye frame; a genuine blink (visor goes fully
dark) replaces the previous static single-tone visor."
```

---

### Task 2: Assembly Line rework

**Files:**
- Modify: `showcase/mouse_grab.rs`
- Modify: `showcase/showcase.rs`

**Interfaces:**
- Consumes: `mascot::MASCOT_WIDTH`/`mascot::MASCOT_HEIGHT` (Task 1's file, values unchanged — `showcase/mascot.rs`'s `pub(crate) const MASCOT_WIDTH: u16 = 12` / `MASCOT_HEIGHT: u16 = 12`), `ttui::transition::Transition`, `ttui::layout::Rect::contains` (both pre-existing).
- Produces: `AssemblyLineState::new(area: Rect) -> Self` — **signature change** from the original argument-free `new() -> Self` (documented deviation below); `AssemblyLineState::mascot_area(&self, area: Rect) -> Rect` — new method, consumed by `showcase.rs`'s `view()`. `on_tick`/`handle_click`/`take_caught`/`is_complete`/`render` keep their existing signatures.

**Why `new` needs `area` now:** the mascot's on-screen position for this vignette is vignette-local state (not the shared top-right position `ShowcaseApp::mascot_area` computes everywhere else), and it must be correctly positioned from the very first rendered frame — before any `on_tick` call, since `view()` renders once immediately when a vignette is entered. `render()` can't lazily compute this itself (it takes `&self`, matching every other vignette's render signature, and can't mutate state). `new()` is therefore the only place left to seed it correctly, so it takes `area` as a parameter. This is a deliberate, one-vignette-only deviation from the plan's normal `new() -> Self` convention — noted explicitly so a reviewer doesn't flag it as an unexplained inconsistency.

- [ ] **Step 1: Rewrite `showcase/mouse_grab.rs`**

Replace the entire file with:

```rust
//! Assembly Line — crates scroll across a fixed lane; clicking one
//! marks it targeted, freezing it in place while the mascot slides
//! over from its current position to grab it. The crate is only
//! actually caught (puff + Grabbing pose + removed) once the
//! mascot's slide animation arrives — not on click. Hit-testing is a
//! real 2D bounding-box check against each crate's own Rect
//! (ttui::layout::Rect::contains), matching control_panel's
//! click-hit-testing pattern; the mascot's on-screen position is
//! owned here (not ShowcaseApp's shared top-right position) since
//! it's vignette-local for this one vignette only.

use crossterm::style::Color;
use std::time::Duration;
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::Rect;
use ttui::particles::{Particle, ParticleSystem};
use ttui::transition::Transition;
use ttui::widgets::text::Text;

use super::mascot;

const CRATE_COUNT: usize = 4;
const CRATE_WIDTH: u16 = 6;
const CRATE_HEIGHT: u16 = 3;
const CRATE_SPEED: f32 = 6.0; // cells/second
const SPAWN_INTERVAL: Duration = Duration::from_millis(1100);
// A fixed span, independent of terminal width — the main lever that
// brings total vignette duration down from the original ~26s to
// roughly double the passive vignettes' scale rather than 5-6x it.
const LANE_TRAVEL_WIDTH: f32 = 50.0;
const PUFF_LIFETIME_MS: u64 = 300;
const MASCOT_SLIDE_DURATION: Duration = Duration::from_millis(300);
// Vertical placement: the mascot sits a couple rows below the
// vignette's top edge, and the crate lane aligns with its claw
// (pixel rows 9-11 of its 12-row sprite), not an arbitrary mid-screen
// row.
const MASCOT_Y_OFFSET: u16 = 2;
const LANE_Y_OFFSET_FROM_MASCOT_TOP: u16 = 9;

fn mascot_y(area: Rect) -> u16 {
    area.y + MASCOT_Y_OFFSET
}

fn lane_y(area: Rect) -> u16 {
    mascot_y(area) + LANE_Y_OFFSET_FROM_MASCOT_TOP
}

fn lane_x0(area: Rect) -> f32 {
    area.x as f32 + (area.width as f32 - LANE_TRAVEL_WIDTH) / 2.0
}

fn palette(code: u8) -> Option<Color> {
    match code {
        10 => Some(Color::Rgb {
            r: 74,
            g: 47,
            b: 26,
        }),
        11 => Some(Color::Rgb {
            r: 199,
            g: 160,
            b: 106,
        }),
        12 => Some(Color::Rgb {
            r: 107,
            g: 114,
            b: 120,
        }),
        _ => None,
    }
}

#[rustfmt::skip]
const CRATE_SPRITE: [[u8; 6]; 3] = [
    [10,10,10,10,10,10],
    [10,11,12,12,11,10],
    [10,10,10,10,10,10],
];

struct CrateItem {
    x: f32,
    targeted: bool,
    caught: bool,
    exited: bool,
}

pub(crate) struct AssemblyLineState {
    crates: Vec<CrateItem>,
    spawn_elapsed: Duration,
    spawned: usize,
    just_caught: bool,
    puff: ParticleSystem,
    lane_y: std::cell::Cell<u16>,
    mascot_x: f32,
    slide_from_x: f32,
    mascot_target_x: f32,
    slide: Transition,
    targeted_crate: Option<usize>,
}

impl AssemblyLineState {
    pub(crate) fn new(area: Rect) -> Self {
        let x0 = lane_x0(area);
        AssemblyLineState {
            crates: Vec::new(),
            spawn_elapsed: Duration::ZERO,
            spawned: 0,
            just_caught: false,
            puff: ParticleSystem::new(),
            lane_y: std::cell::Cell::new(0),
            mascot_x: x0,
            slide_from_x: x0,
            mascot_target_x: x0,
            slide: Transition::start(MASCOT_SLIDE_DURATION),
            targeted_crate: None,
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration, area: Rect) {
        self.puff.update(elapsed);
        let x0 = lane_x0(area);
        let x1 = x0 + LANE_TRAVEL_WIDTH;
        for c in &mut self.crates {
            if c.caught || c.targeted {
                continue; // targeted crates freeze until the mascot arrives
            }
            c.x += CRATE_SPEED * elapsed.as_secs_f32();
            if c.x > x1 {
                c.exited = true;
            }
        }
        self.spawn_elapsed += elapsed;
        if self.spawned < CRATE_COUNT && self.spawn_elapsed >= SPAWN_INTERVAL {
            self.spawn_elapsed = Duration::ZERO;
            self.spawned += 1;
            self.crates.push(CrateItem {
                x: x0,
                targeted: false,
                caught: false,
                exited: false,
            });
        }

        self.slide.tick(elapsed);
        self.mascot_x =
            self.slide_from_x + (self.mascot_target_x - self.slide_from_x) * self.slide.progress();
        if self.slide.is_complete() {
            if let Some(i) = self.targeted_crate.take() {
                self.crates[i].caught = true;
                self.just_caught = true;
                let cx = self.crates[i].x;
                self.puff.spawn(Particle {
                    x: cx + CRATE_WIDTH as f32 / 2.0,
                    y: lane_y(area) as f32,
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
            }
        }
    }

    /// Hit-tests a click against every catchable crate's actual
    /// on-screen `Rect` (cached row from the last `render` call, same
    /// `row_y`-style pattern `control_panel` established, extended to
    /// a full 2D box now that the crate genuinely occupies one).
    /// Retargeting mid-slide abandons the previous target (it resumes
    /// scrolling) rather than special-casing a queue.
    pub(crate) fn handle_click(&mut self, mx: u16, my: u16) {
        let ly = self.lane_y.get();
        let hit = self
            .crates
            .iter()
            .enumerate()
            .find(|(_, c)| {
                !c.caught
                    && !c.exited
                    && !c.targeted
                    && Rect {
                        x: c.x as u16,
                        y: ly,
                        width: CRATE_WIDTH,
                        height: CRATE_HEIGHT,
                    }
                    .contains(mx, my)
            })
            .map(|(i, _)| i);

        if let Some(i) = hit {
            if let Some(prev) = self.targeted_crate {
                self.crates[prev].targeted = false;
            }
            self.crates[i].targeted = true;
            self.targeted_crate = Some(i);
            self.slide_from_x = self.mascot_x;
            self.mascot_target_x = self.crates[i].x;
            self.slide = Transition::start(MASCOT_SLIDE_DURATION);
        }
    }

    /// One-shot: true exactly once, the first call after the mascot's
    /// slide actually arrived at a targeted crate.
    pub(crate) fn take_caught(&mut self) -> bool {
        std::mem::take(&mut self.just_caught)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.spawned == CRATE_COUNT && self.crates.iter().all(|c| c.caught || c.exited)
    }

    /// The mascot's current on-screen area for this vignette —
    /// consumed by `showcase.rs`'s `view()` in place of the shared
    /// `ShowcaseApp::mascot_area`, since the mascot lives inside the
    /// crate lane here rather than its usual top-right spot.
    pub(crate) fn mascot_area(&self, area: Rect) -> Rect {
        Rect {
            x: self.mascot_x as u16,
            y: mascot_y(area),
            width: mascot::MASCOT_WIDTH,
            height: mascot::MASCOT_HEIGHT,
        }
    }

    /// No `theme` parameter — the crate sprite uses its own fixed
    /// wood palette, not the app theme, matching the precedent set by
    /// `OverloadVentState::render` (which also dropped an unused
    /// `theme` param rather than silencing it with `let _ = theme;`).
    pub(crate) fn render(&self, area: Rect, buf: &mut LayerStack) {
        let ly = lane_y(area);
        self.lane_y.set(ly);
        for c in &self.crates {
            if c.caught || c.exited {
                continue;
            }
            let x = c.x as u16;
            for (row, cells) in CRATE_SPRITE.iter().enumerate() {
                let y = ly + row as u16;
                if y >= area.y + area.height {
                    break;
                }
                for (col, &code) in cells.iter().enumerate() {
                    let cx = x + col as u16;
                    if cx >= area.x && cx < area.x + area.width {
                        if let Some(color) = palette(code) {
                            buf.set(
                                cx,
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
        let overlay = buf.push_layer();
        self.puff.render(overlay);
        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Esc back * click a crate").render(hint_row, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 20,
        }
    }

    fn render_to_cache_lane_y(s: &AssemblyLineState) {
        let mut stack = LayerStack::new(100, 20);
        s.render(area(), &mut stack);
    }

    #[test]
    fn a_crate_spawns_at_the_lane_start_after_the_first_spawn_interval() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        assert_eq!(s.spawned, 1);
        assert_eq!(s.crates.len(), 1);
        assert_eq!(s.crates[0].x, lane_x0(area()));
    }

    #[test]
    fn a_crate_past_the_lane_end_is_marked_exited() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        s.on_tick(Duration::from_secs(30), area()); // plenty of time to cross the 50-cell lane
        assert!(s.crates[0].exited);
    }

    #[test]
    fn clicking_a_crate_targets_it_without_immediately_catching_it() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(lane_x0(area()) as u16, ly);
        assert!(s.crates[0].targeted);
        assert!(!s.crates[0].caught);
        assert!(!s.take_caught());
    }

    #[test]
    fn the_slide_completing_actually_catches_the_targeted_crate() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(lane_x0(area()) as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area());
        assert!(s.crates[0].caught);
        assert!(s.take_caught());
    }

    #[test]
    fn take_caught_is_one_shot() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(lane_x0(area()) as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area());
        assert!(s.take_caught());
        assert!(!s.take_caught());
    }

    #[test]
    fn clicking_off_a_crate_does_nothing() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(0, ly); // far from the crate's spawn position
        assert!(!s.crates[0].targeted);
    }

    #[test]
    fn a_targeted_crate_does_not_advance_while_frozen() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        let x_at_click = s.crates[0].x;
        s.handle_click(lane_x0(area()) as u16, ly);
        s.on_tick(Duration::from_millis(100), area()); // well under MASCOT_SLIDE_DURATION
        assert_eq!(s.crates[0].x, x_at_click);
    }

    #[test]
    fn retargeting_mid_slide_abandons_the_previous_target() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(s.crates[0].x as u16, ly);
        assert!(s.crates[0].targeted);
        // Retarget onto crate 1 before crate 0's slide completes.
        s.handle_click(s.crates[1].x as u16, ly);
        assert!(!s.crates[0].targeted, "abandoned target should resume scrolling");
        assert!(s.crates[1].targeted);
    }

    #[test]
    fn is_complete_once_all_spawned_crates_are_caught_or_exited() {
        let mut s = AssemblyLineState::new(area());
        for _ in 0..CRATE_COUNT {
            s.on_tick(SPAWN_INTERVAL, area());
        }
        assert_eq!(s.spawned, CRATE_COUNT);
        s.on_tick(Duration::from_secs(30), area()); // everything exits
        assert!(s.is_complete());
    }

    #[test]
    fn not_complete_until_every_crate_has_spawned() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        assert!(!s.is_complete());
    }

    #[test]
    fn mascot_area_tracks_the_current_slide_position() {
        let mut s = AssemblyLineState::new(area());
        let initial = s.mascot_area(area());
        assert_eq!(initial.x, lane_x0(area()) as u16);
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(s.crates[0].x as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area());
        let after = s.mascot_area(area());
        assert_eq!(after.x, s.crates[0].x as u16);
    }
}
```

- [ ] **Step 2: Update `showcase/showcase.rs`'s Assembly Line wiring**

In `enter_vignette`, change the `AssemblyLine` arm from:

```rust
            VignetteId::AssemblyLine => self.assembly_line = Some(AssemblyLineState::new()),
```

to:

```rust
            VignetteId::AssemblyLine => {
                self.assembly_line = Some(AssemblyLineState::new(self.last_area.get()))
            }
```

In `view()`, change the `Screen::Vignette(VignetteId::AssemblyLine)` arm from:

```rust
            Screen::Vignette(VignetteId::AssemblyLine) => {
                if let Some(state) = &self.assembly_line {
                    state.render(area, &self.theme, buf);
                }
                self.mascot.render(mascot_area, buf);
            }
```

to:

```rust
            Screen::Vignette(VignetteId::AssemblyLine) => {
                if let Some(state) = &self.assembly_line {
                    state.render(area, buf);
                    self.mascot.render(state.mascot_area(area), buf);
                }
            }
```

(Note this drops the `&self.theme` argument `AssemblyLineState::render` used to take — Task 2 Step 1's rewrite removes that now-unused parameter. The shared `mascot_area` local variable computed at the top of `view()` is still used by the other mascot-showing vignette arms — `OverrideSequence` — so don't remove it; only Assembly Line's arm stops using it.)

`on_tick()`'s `Screen::Vignette(VignetteId::AssemblyLine)` arm needs no change — it already calls `state.on_tick(elapsed, area)`, `state.take_caught()`, `state.is_complete()` in that order, which is exactly what the reworked state still expects.

- [ ] **Step 3: Run tests**

Run: `cargo test --bin showcase`
Expected: all of Task 1's tests plus this task's 11 new tests pass (the 6 pre-existing `mouse_grab` tests are replaced by the 11 above, matching the reworked behavior — not additive on top of the old ones, since the old tests assert the old exact-catch-on-click semantics this task replaces).

- [ ] **Step 4: Build and lint**

Run: `cargo build --bin showcase`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
Expected: all clean.

- [ ] **Step 5: Capture and verify visually — targeting and the slide**

```json
[{"wait_ms": 1500}, {"key": "Enter"}, {"wait_ms": 1300}, {"x": 25, "y": 13}, {"wait_ms": 100}, {"wait_ms": 100}, {"wait_ms": 200}]
```

(`Enter` from tile 0, "Assembly Line"; adjust the click `x`/`y` based on an intermediate capture showing exactly where the first crate and the lane row actually land at `100x30` — same empirical-coordinate approach used throughout the original Arc's plan.) `Read` the resulting frames (extract with ffmpeg/PIL for later frames — `Read` only shows frame 1 of a multi-frame GIF). Confirm: a wood-crate sprite (not a `#` row) is visible mid-lane, the mascot is positioned within the lane (not top-right), and across the frames after the click the mascot's x-position visibly moves toward the crate before the crate disappears with a puff.

- [ ] **Step 6: Capture and verify visually — retargeting**

Extend or write a second script that clicks two different crates in quick succession (before the first slide would complete) and confirms the mascot redirects to the second crate rather than continuing toward the first, per the design's "no special-casing" retarget behavior.

- [ ] **Step 7: Commit**

Do not manually close issue #128 with `gh issue close` — the commit
message below includes `Closes #128.`, which GitHub auto-closes on
merge (the same pattern already established for #115/#119 in this
project). Closing it manually now would close it prematurely, before
this Arc's PR even merges.

```bash
git add showcase/mouse_grab.rs showcase/showcase.rs
git commit -m "feat(showcase): rework Assembly Line — real crate, mascot slides to catch

Replaces the plain '#'-row crate with a real pixel-tile sprite, and
the mascot now slides into the crate lane and travels to whichever
crate is clicked before the catch registers (puff + Grabbing pose),
rather than pose-flashing from a stationary top-right position. Hit-
testing becomes a real 2D Rect::contains check against the crate's
actual bounds, replacing the old single-row + tolerance check.

Retunes CRATE_COUNT/WIDTH/SPEED/SPAWN_INTERVAL and confines the lane
to a fixed 50-cell span (was the full terminal width), bringing the
vignette's total duration down from ~26s to ~12s and fixing the
chain-overlap the previous speed/width ratio caused.

Closes #128."
```

---

### Task 3: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Full workspace build, lint, format, test**

Run: `cargo build --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test`
Expected: all clean; full workspace suite green, including Task 1's 8 new mascot tests and Task 2's 11 new/replaced `mouse_grab` tests.

- [ ] **Step 2: Deliver the human-only checklist**

Present this exact checklist to the user (do not paraphrase or shorten it):

1. Open a real terminal window — not through Claude Code.
2. `cd` to this worktree.
3. `cargo run --bin showcase`. Wait at the menu for at least 5-6 seconds without pressing anything — confirm the mascot's antenna visibly dips and recovers on its own (breathing), and confirm you see at least one moment where its visor goes fully dark briefly (blinking).
4. Press Enter on "Assembly Line". Click a crate and confirm the mascot visibly slides across the lane to reach it before the catch happens (particle puff + crate disappears) — it should NOT feel instant/teleporting. Try clicking a second crate while a previous catch animation might still be settling, and confirm the mascot redirects smoothly rather than glitching. Press `Esc` to return, then `q` to quit. Confirm the terminal returns to a normal shell prompt — cursor visible, no leftover alternate-screen artifacts. Type any command (e.g. `ls`) afterward and confirm it echoes and behaves normally.
5. Report back here: did everything behave as described? If anything looked wrong, stuck, garbled, felt too fast/slow, or the terminal didn't restore cleanly, describe exactly what happened and which step it happened on.

- [ ] **Step 3: Wait for the user's report**

Do not mark this task complete until the user has actually responded with their observations.

- [ ] **Step 4: Record the result**

No additional commit for this step. If the user reports any problem, file it as a GitHub issue and triage it per `code-forge.md`'s rule (same as the original Arc). If zero findings resulted, record that plainly — a valid, complete outcome.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — all clean.
- [ ] `cargo test` — full suite green, including all of Task 1's and Task 2's new tests.
- [ ] Every `tools/visual-snapshot` capture across Tasks 1-2 was actually `Read` and reviewed.
- [ ] Task 2 Step 7's commit includes `Closes #128.` (the issue itself closes automatically once this Arc's PR merges to `main`, not before).
- [ ] The human-only checklist (Task 3) was delivered verbatim and the user's report was actually received, not skipped.
- [ ] Any genuine finding from Task 3 is filed as a GitHub issue with the correct `semver:*`/`v1-blocking` labels — `showcase/` isn't part of `ttui`'s public API surface, so findings default to `semver:patch`, no `v1-blocking`.
- [ ] Per `.claude/rules/git-github-standards.md`: this Arc is `coding`-tagged (Gated tier) — open a PR from this worktree's branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree`. Note: PR #129 (the base Arc this one builds on) may or may not have merged first — if it hasn't, this PR's diff will include #129's commits too; call that out explicitly in the PR description rather than letting it look like scope creep.
