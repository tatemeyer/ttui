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
        assert!(
            !s.crates[0].targeted,
            "abandoned target should resume scrolling"
        );
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
