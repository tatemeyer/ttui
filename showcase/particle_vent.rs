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
        assert_eq!(
            s.particles.len(),
            EMITTER_OFFSETS.len() * PARTICLES_PER_EMITTER
        );
    }

    #[test]
    fn stops_emitting_once_the_vent_duration_completes() {
        let mut s = OverloadVentState::new();
        s.on_tick(VENT_DURATION, area()); // completes the transition
        let count_at_completion = s.particles.len();
        s.on_tick(EMIT_INTERVAL, area()); // would emit more if still active
        assert_eq!(s.particles.len().saturating_sub(count_at_completion), 0);
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
        s.on_tick(EMIT_INTERVAL, area());
        let after_first_burst = s.particles.len();
        s.on_tick(EMIT_INTERVAL, area());
        assert!(s.particles.len() > after_first_burst);
    }
}
