//! Assembly Line — crates scroll across a fixed lane; clicking one
//! marks it targeted, freezing it in place while the mascot slides
//! horizontally from its current position to above the target. Once
//! that horizontal slide arrives, the mascot reaches its claw down
//! into the lane (a separate, vertical `Transition`), and only then
//! is the crate actually caught (puff + removed) — not at click, and
//! not merely at slide-arrival either. The whole line pauses while
//! the claw is reaching down or retracting, so nothing drifts under
//! it mid-reach. Hit-testing is a real 2D bounding-box check against
//! each crate's own Rect (ttui::layout::Rect::contains), matching
//! control_panel's click-hit-testing pattern; the mascot's on-screen
//! position is owned here (not ShowcaseApp's shared top-right
//! position) since it's vignette-local for this one vignette only.
//! The lane sits entirely below the mascot's own sprite at rest (see
//! `LANE_Y_OFFSET_FROM_MASCOT_TOP`) so the two never overlap except
//! during the deliberate reach-down motion.

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
// vignette's top edge, and the crate lane sits one full row below the
// mascot's entire 12-row sprite — never overlapping it at rest. The
// claw only visits the lane during the deliberate reach-down motion
// below (`REACH_DOWN_OFFSET`).
const MASCOT_Y_OFFSET: u16 = 2;
const LANE_Y_OFFSET_FROM_MASCOT_TOP: u16 = 13;
// How far (in rows) and how long the claw reaches down into the lane
// after the horizontal slide arrives, before the catch fires and it
// retracts. Reuses the same `Transition` pattern as the horizontal
// slide, just driving a vertical offset instead of an x-position.
const REACH_DOWN_OFFSET: u16 = 4;
const REACH_DURATION: Duration = Duration::from_millis(200);

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

/// Tracks the mascot's claw through its post-slide vertical motion:
/// `None` while at rest or still sliding horizontally, `Down` while
/// reaching into the lane (catch fires when this completes), `Up`
/// while retracting back to the resting height.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ReachPhase {
    None,
    Down,
    Up,
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
    reach_phase: ReachPhase,
    reach: Transition,
    mascot_y_offset: f32,
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
            reach_phase: ReachPhase::None,
            reach: Transition::start(REACH_DURATION),
            mascot_y_offset: 0.0,
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration, area: Rect) {
        self.puff.update(elapsed);
        let x0 = lane_x0(area);
        let x1 = x0 + LANE_TRAVEL_WIDTH;
        // While the claw is reaching down or retracting, the whole
        // line holds still — not just the targeted crate — so nothing
        // drifts underneath a claw that's visually in the lane.
        let line_paused = self.reach_phase != ReachPhase::None;
        if !line_paused {
            for c in &mut self.crates {
                if c.caught || c.targeted {
                    continue; // targeted crates freeze until the mascot arrives
                }
                c.x += CRATE_SPEED * elapsed.as_secs_f32();
                if c.x > x1 {
                    c.exited = true;
                }
            }
            // The spawn clock must pause too, not just movement — this
            // is the same "whole line holds still" contract, and
            // letting it run during a reach cycle silently compresses
            // the next crate's spacing below CRATE_WIDTH (reopening
            // #128's chain-overlap on essentially every catch).
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
        }

        self.slide.tick(elapsed);
        self.mascot_x =
            self.slide_from_x + (self.mascot_target_x - self.slide_from_x) * self.slide.progress();

        match self.reach_phase {
            ReachPhase::None => {
                // Horizontal slide has arrived above the target: begin
                // reaching the claw down. The freshly started
                // `Transition` isn't ticked until the next call, same
                // as `slide` isn't ticked in the same call
                // `handle_click` starts it.
                if self.slide.is_complete() && self.targeted_crate.is_some() {
                    self.reach_phase = ReachPhase::Down;
                    self.reach = Transition::start(REACH_DURATION);
                }
            }
            ReachPhase::Down => {
                self.reach.tick(elapsed);
                self.mascot_y_offset = REACH_DOWN_OFFSET as f32 * self.reach.progress();
                if self.reach.is_complete() {
                    // The claw has arrived at the lane — catch fires
                    // here, not at horizontal slide-arrival.
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
                    self.reach_phase = ReachPhase::Up;
                    self.reach = Transition::start(REACH_DURATION);
                }
            }
            ReachPhase::Up => {
                self.reach.tick(elapsed);
                self.mascot_y_offset = REACH_DOWN_OFFSET as f32 * (1.0 - self.reach.progress());
                if self.reach.is_complete() {
                    self.mascot_y_offset = 0.0;
                    self.reach_phase = ReachPhase::None;
                }
            }
        }
    }

    /// Hit-tests a click against every catchable crate's actual
    /// on-screen `Rect` (cached row from the last `render` call, same
    /// `row_y`-style pattern `control_panel` established, extended to
    /// a full 2D box now that the crate genuinely occupies one).
    /// Retargeting during the horizontal slide phase abandons the
    /// previous target (it resumes scrolling) rather than
    /// special-casing a queue; retargeting during the `Down`/`Up`
    /// reach phases has a known, separately tracked issue (#132) and
    /// is left alone here.
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
            // Abandon any in-flight reach before retargeting (#132).
            // `slide` restarts below, but `reach` would otherwise keep
            // running on the *old* target's timeline — and the catch in
            // `ReachPhase::Down` fires against whatever `targeted_crate`
            // names *then*, which by that point is this new crate. The
            // mascot would catch it without having slid to it.
            //
            // Only `Down` needs handling: `Up` never calls `take()`, so
            // retargeting during a retract is already safe, and letting
            // it finish retracting is the right behaviour anyway.
            if self.reach_phase == ReachPhase::Down {
                // Retract from where the claw actually is rather than
                // snapping to full depth first: `Up` drives the offset
                // as `OFFSET * (1 - progress)`, so entering it at
                // `1 - down_progress` picks up at the same height.
                let down_progress = self.reach.progress();
                self.reach = Transition::start(REACH_DURATION);
                self.reach.tick(Duration::from_secs_f32(
                    (1.0 - down_progress) * REACH_DURATION.as_secs_f32(),
                ));
                self.reach_phase = ReachPhase::Up;
            }
            if let Some(prev) = self.targeted_crate {
                self.crates[prev].targeted = false;
            }
            self.crates[i].targeted = true;
            self.targeted_crate = Some(i);
            self.slide_from_x = self.mascot_x;
            // Center the claw (roughly mid-sprite) on the crate's own
            // center, not the sprite's left edge on the crate's left
            // edge — otherwise the claw lands on the crate's right
            // edge/corner instead of its middle, visibly offset from
            // where the puff (spawned at the crate's true center)
            // appears.
            self.mascot_target_x =
                self.crates[i].x + CRATE_WIDTH as f32 / 2.0 - mascot::MASCOT_WIDTH as f32 / 2.0;
            self.slide = Transition::start(MASCOT_SLIDE_DURATION);
        }
    }

    /// One-shot: true exactly once, the first call after the mascot's
    /// reach-down actually arrived at a targeted crate (not merely
    /// after the horizontal slide arrives above it).
    pub(crate) fn take_caught(&mut self) -> bool {
        std::mem::take(&mut self.just_caught)
    }

    /// Also requires `reach_phase == None`, not just every crate
    /// caught/exited — otherwise catching the *last* crate would flip
    /// this true on the exact tick the catch fires, and `showcase.rs`
    /// would call `exit_vignette()` before the retract animation, the
    /// `Grabbing` pose, or the freshly spawned puff ever render.
    pub(crate) fn is_complete(&self) -> bool {
        self.spawned == CRATE_COUNT
            && self.crates.iter().all(|c| c.caught || c.exited)
            && self.reach_phase == ReachPhase::None
    }

    /// The mascot's current on-screen area for this vignette —
    /// consumed by `showcase.rs`'s `view()` in place of the shared
    /// `ShowcaseApp::mascot_area`, since the mascot lives inside the
    /// crate lane here rather than its usual top-right spot.
    pub(crate) fn mascot_area(&self, area: Rect) -> Rect {
        Rect {
            x: self.mascot_x as u16,
            y: mascot_y(area) + self.mascot_y_offset as u16,
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

    /// The horizontal slide arriving is only the halfway point now —
    /// the claw still has to reach down before the catch fires. This
    /// replaces the old "slide completing catches immediately"
    /// behavior removed by the reach-down/pause fix (issue found in
    /// live interactive testing of commit 7fa27ae: the lane used to
    /// overlap the mascot's own sprite at rest).
    #[test]
    fn the_slide_arriving_alone_does_not_yet_catch_the_targeted_crate() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(lane_x0(area()) as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area());
        assert!(!s.crates[0].caught);
        assert!(!s.take_caught());
        assert_eq!(s.reach_phase, ReachPhase::Down);
    }

    #[test]
    fn the_reach_down_phase_completing_actually_catches_the_targeted_crate() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(lane_x0(area()) as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area()); // horizontal slide arrives, reach-down begins
        s.on_tick(REACH_DURATION, area()); // reach-down completes
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
        s.on_tick(REACH_DURATION, area());
        assert!(s.take_caught());
        assert!(!s.take_caught());
    }

    #[test]
    fn the_lane_sits_entirely_below_the_mascot_sprite_at_rest() {
        // Regression guard for the overlap found in live testing: the
        // lane's top row must be at or below the mascot's full height,
        // never inside its 12-row sprite.
        assert!(lane_y(area()) >= mascot_y(area()) + mascot::MASCOT_HEIGHT);
    }

    #[test]
    fn reach_down_moves_the_mascot_offset_toward_full_depth() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(lane_x0(area()) as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area()); // reach-down begins, offset still 0
        assert_eq!(s.mascot_y_offset, 0.0);
        s.on_tick(REACH_DURATION / 2, area()); // halfway into the reach
        assert!(s.mascot_y_offset > 0.0 && s.mascot_y_offset < REACH_DOWN_OFFSET as f32);
        s.on_tick(REACH_DURATION / 2, area()); // reach-down completes
        assert_eq!(s.mascot_y_offset, REACH_DOWN_OFFSET as f32);
        assert_eq!(s.reach_phase, ReachPhase::Up);
    }

    #[test]
    fn the_retract_phase_returns_the_mascot_offset_to_zero() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(lane_x0(area()) as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area()); // reach-down begins
        s.on_tick(REACH_DURATION, area()); // reach-down completes, retract begins
        assert_eq!(s.reach_phase, ReachPhase::Up);
        s.on_tick(REACH_DURATION, area()); // retract completes
        assert_eq!(s.mascot_y_offset, 0.0);
        assert_eq!(s.reach_phase, ReachPhase::None);
    }

    #[test]
    fn non_targeted_crates_freeze_while_the_claw_is_reaching_then_resume() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(s.crates[0].x as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area()); // reach-down begins; line pauses
        let x_during_reach = s.crates[1].x;
        s.on_tick(Duration::from_millis(50), area()); // still within REACH_DURATION
        assert_eq!(
            s.crates[1].x, x_during_reach,
            "non-targeted crate should not drift while the claw is reaching"
        );
        s.on_tick(REACH_DURATION, area()); // reach-down completes, retract begins
        assert_eq!(s.crates[1].x, x_during_reach, "still paused during retract");
        s.on_tick(REACH_DURATION, area()); // retract completes, line resumes
        s.on_tick(Duration::from_millis(50), area());
        assert!(
            s.crates[1].x > x_during_reach,
            "crate should resume scrolling once the reach phase returns to None"
        );
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
        // The claw (roughly mid-sprite) centers on the crate's own
        // center, not the sprite's left edge on the crate's left edge.
        let expected_x =
            s.crates[0].x + CRATE_WIDTH as f32 / 2.0 - mascot::MASCOT_WIDTH as f32 / 2.0;
        assert_eq!(after.x, expected_x as u16);
    }

    #[test]
    fn reach_down_offset_is_deep_enough_to_reach_the_lane() {
        // Regression guard coupling the reach depth to the lane gap:
        // shrinking REACH_DOWN_OFFSET (or growing the gap) without
        // updating the other would silently leave the claw short of
        // the lane it's supposed to reach into.
        assert!(mascot_y(area()) + REACH_DOWN_OFFSET + mascot::MASCOT_HEIGHT > lane_y(area()));
    }

    #[test]
    fn a_full_catch_cycle_does_not_compress_spacing_below_crate_width() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        s.handle_click(s.crates[0].x as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area()); // horizontal slide arrives, reach-down begins; pause starts now
        let follower_x_when_pause_begins = s.crates[1].x;
        let spawn_elapsed_when_pause_begins = s.spawn_elapsed;
        s.on_tick(REACH_DURATION, area()); // catch fires, reach_phase -> Up
        s.on_tick(REACH_DURATION, area()); // retract completes, reach_phase -> None
                                           // Both the follower crate's position AND the spawn clock must
                                           // be exactly what they were when the pause began — not just
                                           // "the crate didn't move" (movement was already correctly
                                           // gated before this fix), but specifically that
                                           // `spawn_elapsed` itself didn't keep advancing during the
                                           // 400ms (REACH_DURATION * 2) reach-down/retract cycle. Before
                                           // the fix, `spawn_elapsed` kept accumulating during the pause,
                                           // silently compressing the next crate's spacing below
                                           // CRATE_WIDTH once movement resumed.
        assert_eq!(s.crates[1].x, follower_x_when_pause_begins);
        assert_eq!(s.spawn_elapsed, spawn_elapsed_when_pause_begins);
    }

    /// #132: `handle_click` restarted `slide` but left `reach_phase`,
    /// `reach` and `mascot_y_offset` alone. Retargeting mid-reach-down
    /// therefore let the *old* target's reach run to completion on its
    /// own timeline, and `targeted_crate.take()` by then named the *new*
    /// crate — catching it before the mascot had slid anywhere near it.
    #[test]
    fn retargeting_mid_reach_does_not_catch_the_new_crate_early() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());
        assert!(s.crates.len() >= 2, "need two crates to retarget between");

        s.handle_click(s.crates[0].x as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area()); // slide arrives, reach-down begins
        s.on_tick(REACH_DURATION / 2, area()); // halfway down
        assert_eq!(s.reach_phase, ReachPhase::Down, "precondition: mid-reach");

        // A second, untargeted crate — legal to click, since the whole
        // line is paused and it is sitting still.
        let second = s.crates[1].x as u16;
        s.handle_click(second, ly);

        // Exactly the remainder of the *old* reach. Before the fix this
        // completed that reach and caught crate 1 on the spot.
        s.on_tick(REACH_DURATION / 2, area());

        assert!(
            !s.crates[1].caught,
            "the newly targeted crate must not be caught by the previous \
             target's in-flight reach"
        );
        assert!(
            !s.crates[0].caught,
            "the abandoned target must not be caught either"
        );
    }

    /// The abandoned reach retracts from where the claw actually is.
    /// Entering `Up` with a fresh `Transition` would drive the offset as
    /// `OFFSET * (1 - 0)` — i.e. snap the claw to full depth first and
    /// only then retract, a visible jump downward at the moment the
    /// player clicks somewhere else.
    #[test]
    fn retargeting_mid_reach_retracts_from_the_claws_current_depth() {
        let mut s = AssemblyLineState::new(area());
        s.on_tick(SPAWN_INTERVAL, area());
        s.on_tick(SPAWN_INTERVAL, area());
        render_to_cache_lane_y(&s);
        let ly = lane_y(area());

        s.handle_click(s.crates[0].x as u16, ly);
        s.on_tick(MASCOT_SLIDE_DURATION, area());
        s.on_tick(REACH_DURATION / 2, area()); // ~halfway down
        let depth_before = s.mascot_y_offset;
        assert!(
            depth_before > 0.0 && depth_before < REACH_DOWN_OFFSET as f32,
            "precondition: claw partway down, got {depth_before}"
        );

        s.handle_click(s.crates[1].x as u16, ly);
        s.on_tick(Duration::ZERO, area()); // recompute offset, advance nothing

        assert!(
            s.mascot_y_offset <= depth_before + 0.01,
            "retract must continue from {depth_before}, not snap deeper \
             (got {})",
            s.mascot_y_offset
        );
    }
}
