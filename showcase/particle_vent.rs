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
}
