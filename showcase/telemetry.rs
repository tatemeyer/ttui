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

        let stats = [
            ("Power Draw", self.power_draw),
            ("Efficiency", self.efficiency),
        ];
        let stats_inner = Block::new()
            .title("Stats")
            .theme(theme)
            .render(cols[1], buf);
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
