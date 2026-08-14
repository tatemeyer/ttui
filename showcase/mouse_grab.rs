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
// Tuned for real human reaction time, not just script-precise clicks: at
// the original 10.0 cells/sec + 6-wide crate + exact-row hit test, a
// crate was only catchable for ~0.6s on one exact terminal row — fine
// for a scripted visual-snapshot click computed from a static frame,
// not for someone watching a moving target and clicking in real time.
const CRATE_SPEED: f32 = 4.5; // cells/second
const CRATE_WIDTH: u16 = 8;
// `handle_click`'s row hit-test accepts this many rows above/below the
// cached `row_y`, absorbing real mouse/terminal imprecision (a human
// aiming at "the crate's row" by eye won't always land the exact cell).
const ROW_TOLERANCE: u16 = 1;
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
        // Do NOT reset `just_caught` here: `take_caught()` (`mem::take`)
        // is the sole place that consumes it. `handle_click` (fired from
        // `update`) always runs in a separate event from `on_tick`, so a
        // reset here would always clear a catch before the app's own
        // `on_tick` gets a chance to call `take_caught()` after this call
        // returns — silently swallowing every catch.
        self.puff.update(elapsed);
        // Move existing crates first, then spawn: a freshly spawned crate
        // must not also advance in the same tick it's pushed, or it lands
        // past `area.x` immediately (breaking hit-testing against the
        // spawn position for that frame).
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
    }

    /// Hit-tests a click against the row cached from the last
    /// `render` call — mirrors `control_panel`'s `button_area`
    /// pattern (a Cell populated at render time, read at click time).
    /// Accepts `ROW_TOLERANCE` rows of slop either side of the exact
    /// row, not just a pixel-perfect match.
    pub(crate) fn handle_click(&mut self, mx: u16, my: u16) {
        if my.abs_diff(self.row_y.get()) > ROW_TOLERANCE {
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

    /// Regression test for a real integration bug: `ShowcaseApp::on_tick`
    /// (showcase.rs) calls `state.on_tick(..)` and THEN
    /// `state.take_caught()` in the same call — the ordering the real
    /// app always uses, since `handle_click` fires from a separate
    /// `update` event, never inside `on_tick` itself. An earlier draft
    /// reset `just_caught` at the top of `on_tick`, which silently
    /// discarded every catch before the app could ever observe it.
    #[test]
    fn caught_flag_survives_an_on_tick_call_before_being_read() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        let theme = Theme::default();
        let mut stack = LayerStack::new(40, 10);
        s.render(area(), &theme, &mut stack);
        let row_y = area().y + area().height / 2;
        s.handle_click(0, row_y);
        s.on_tick(Duration::from_millis(33), area());
        assert!(s.take_caught());
    }

    #[test]
    fn clicking_off_row_does_not_catch() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        let theme = Theme::default();
        let mut stack = LayerStack::new(40, 10);
        s.render(area(), &theme, &mut stack);
        s.handle_click(0, 0); // wrong row, well outside ROW_TOLERANCE
        assert!(!s.crates[0].caught);
    }

    /// `handle_click`'s row hit-test forgives `ROW_TOLERANCE` rows of
    /// slop either side of the cached row — a human aiming by eye at a
    /// scrolling row won't always land the exact cell.
    #[test]
    fn clicking_one_row_off_within_tolerance_still_catches() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        let theme = Theme::default();
        let mut stack = LayerStack::new(40, 10);
        s.render(area(), &theme, &mut stack);
        let row_y = area().y + area().height / 2;
        s.handle_click(0, row_y + ROW_TOLERANCE); // one row below, still in tolerance
        assert!(s.crates[0].caught);
    }

    #[test]
    fn clicking_beyond_row_tolerance_does_not_catch() {
        let mut s = AssemblyLineState::new();
        s.on_tick(SPAWN_INTERVAL, area());
        let theme = Theme::default();
        let mut stack = LayerStack::new(40, 10);
        s.render(area(), &theme, &mut stack);
        let row_y = area().y + area().height / 2;
        s.handle_click(0, row_y + ROW_TOLERANCE + 1); // one row past tolerance
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
