//! Overload Vent — 3 simultaneous particle emitters vent for a fixed
//! duration, exercising particles.rs more fully than control_panel's
//! single-button burst does. No interaction required.
//!
//! Tuned for real-time legibility (2026-08-14 fix): the original
//! numbers (1 particle/emitter/80ms, 350ms lifetime, tight ±4-cell
//! offsets) read as invisible/too-subtle to a live viewer even though
//! individual captured frames technically showed particles when
//! inspected pixel-by-pixel. Density, lifetime, spread, and velocity
//! were all increased so the vent reads as an obvious burst rather
//! than a faint flicker — see the Task 6 fix report.

use crossterm::style::Color;
use std::time::Duration;
use ttui::buffer::LayerStack;
use ttui::layout::Rect;
use ttui::particles::{Particle, ParticleSystem};
use ttui::transition::Transition;
use ttui::widgets::text::Text;

const VENT_DURATION: Duration = Duration::from_millis(3500);
const EMIT_INTERVAL: Duration = Duration::from_millis(60);
const SPARK_LIFETIME_MS: u64 = 600;
const PARTICLES_PER_EMITTER: usize = 2;
const EMITTER_OFFSETS: [(f32, f32); 3] = [(-8.0, -3.0), (0.0, -5.0), (8.0, -3.0)];

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

    /// `mascot_area` is the same rect `showcase.rs` renders the mascot
    /// sprite into — emitters anchor near its shoulder/joint band (a
    /// few rows down from the top) so the vent visibly emanates from
    /// the mascot rather than raw screen center.
    pub(crate) fn on_tick(&mut self, elapsed: Duration, mascot_area: Rect) {
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
        let cx = mascot_area.x as f32 + mascot_area.width as f32 / 2.0;
        let cy = mascot_area.y as f32 + 3.0;
        for (i, &(ox, oy)) in EMITTER_OFFSETS.iter().enumerate() {
            let base_angle = (i as f32 / EMITTER_OFFSETS.len() as f32) * std::f32::consts::TAU
                + self.transition.progress() * std::f32::consts::TAU * 4.0;
            for j in 0..PARTICLES_PER_EMITTER {
                // Fan each emitter's simultaneous particles out by a
                // small deterministic angle offset so a burst reads as
                // a spread of sparks rather than overlapping glyphs.
                let fan = (j as f32 - (PARTICLES_PER_EMITTER as f32 - 1.0) / 2.0) * 0.5;
                let angle = base_angle + fan;
                self.particles.spawn(Particle {
                    x: cx + ox,
                    y: cy + oy,
                    vx: angle.cos() * 10.0,
                    vy: angle.sin() * 5.0 - 3.0,
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
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.transition.is_complete() && self.particles.is_empty()
    }

    pub(crate) fn render(&self, area: Rect, buf: &mut LayerStack) {
        let overlay = buf.push_layer();
        self.particles.render(overlay);
        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Esc back").render(hint_row, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Stands in for the mascot's on-screen rect (the same shape
    // showcase.rs computes via `ShowcaseApp::mascot_area`), not a full
    // screen area — `on_tick` anchors emitters relative to this.
    fn mascot_area() -> Rect {
        Rect {
            x: 26,
            y: 1,
            width: 12,
            height: 12,
        }
    }

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
        s.on_tick(EMIT_INTERVAL, mascot_area());
        assert_eq!(
            s.particles.len(),
            EMITTER_OFFSETS.len() * PARTICLES_PER_EMITTER
        );
    }

    #[test]
    fn stops_emitting_once_the_vent_duration_completes() {
        let mut s = OverloadVentState::new();
        s.on_tick(VENT_DURATION, mascot_area()); // completes the transition
        let count_at_completion = s.particles.len();
        s.on_tick(EMIT_INTERVAL, mascot_area()); // would emit more if still active
        assert_eq!(s.particles.len().saturating_sub(count_at_completion), 0);
    }

    #[test]
    fn is_complete_once_duration_elapses_and_particles_fade() {
        let mut s = OverloadVentState::new();
        s.on_tick(VENT_DURATION, mascot_area());
        s.on_tick(Duration::from_secs(2), mascot_area()); // long enough for sparks to expire
        assert!(s.is_complete());
    }

    #[test]
    fn not_complete_while_duration_is_still_running() {
        let mut s = OverloadVentState::new();
        s.on_tick(Duration::from_millis(100), mascot_area());
        assert!(!s.is_complete());
    }

    /// Regression guard for the 2026-08-14 legibility fix: with
    /// SPARK_LIFETIME_MS (600ms) more than double EMIT_INTERVAL (60ms),
    /// a burst's particles must still be alive when the *next* burst
    /// spawns, so the on-screen population grows across consecutive
    /// emissions instead of flickering back toward zero between them —
    /// which is what made the original 350ms/80ms pairing read as an
    /// invisible flicker to a live viewer despite passing captures.
    #[test]
    fn particle_population_grows_across_consecutive_bursts() {
        let mut s = OverloadVentState::new();
        s.on_tick(EMIT_INTERVAL, mascot_area());
        let after_first_burst = s.particles.len();
        s.on_tick(EMIT_INTERVAL, mascot_area());
        assert!(s.particles.len() > after_first_burst);
    }

    /// Regression guard for the final-review fix (2026-08-14): emitters
    /// must anchor near the mascot's rect, not a separately-passed
    /// screen area — moving `mascot_area()` far from origin must still
    /// spawn the same particle count without panicking on out-of-range
    /// coordinates.
    #[test]
    fn emitting_relative_to_a_mascot_area_far_from_origin_still_spawns_particles() {
        let mut s = OverloadVentState::new();
        let far_mascot_area = Rect {
            x: 200,
            y: 50,
            width: 12,
            height: 12,
        };
        s.on_tick(EMIT_INTERVAL, far_mascot_area);
        assert_eq!(
            s.particles.len(),
            EMITTER_OFFSETS.len() * PARTICLES_PER_EMITTER
        );
    }

    #[test]
    fn render_draws_the_esc_back_hint_on_the_bottom_row() {
        let s = OverloadVentState::new();
        let mut stack = LayerStack::new(area().width, area().height);
        s.render(area(), &mut stack);
        let bottom_row = area().height - 1;
        assert_eq!(stack.get(0, bottom_row).symbol, 'E');
        assert_eq!(stack.get(1, bottom_row).symbol, 's');
        assert_eq!(stack.get(2, bottom_row).symbol, 'c');
    }
}
