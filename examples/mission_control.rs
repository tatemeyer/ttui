use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::LayerStack;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{bar_chart::BarChart, block::Block, sparkline::Sparkline};

const TICK_INTERVAL: Duration = Duration::from_millis(33);
const HISTORY_LEN: usize = 200;
const ALTITUDE_STEP: f32 = 15.0;
const VELOCITY_STEP: f32 = 8.0;
const SIGNAL_STEP: f32 = 4.0;
const SUBSYSTEM_STEP: f32 = 3.0;

const SUBSYSTEM_NAMES: [&str; 5] = ["Engines", "Life Support", "Comms", "Nav", "Power"];

fn mission_control_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 5, g: 10, b: 15 },
        primary: Color::Rgb {
            r: 80,
            g: 200,
            b: 255,
        },
        secondary: Color::Rgb {
            r: 230,
            g: 230,
            b: 230,
        },
        tertiary: Color::Rgb {
            r: 255,
            g: 60,
            b: 60,
        },
        accent: Color::Rgb {
            r: 255,
            g: 180,
            b: 60,
        },
        primary_end: None,
        border: BorderSet::default(),
        border_bold: false,
        border_thick: false,
    }
}

fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}

struct MissionControl {
    theme: Theme,
    altitude: f32,
    altitude_history: Vec<f32>,
    velocity: f32,
    velocity_history: Vec<f32>,
    signal: f32,
    signal_history: Vec<f32>,
    subsystems: [f32; 5],
    tick_count: u64,
    quit: bool,
}

impl MissionControl {
    fn new() -> Self {
        MissionControl {
            theme: mission_control_theme(),
            altitude: 5000.0,
            altitude_history: vec![5000.0],
            velocity: 250.0,
            velocity_history: vec![250.0],
            signal: 80.0,
            signal_history: vec![80.0],
            subsystems: [95.0, 92.0, 88.0, 90.0, 97.0],
            tick_count: 0,
            quit: false,
        }
    }

    fn push_history(history: &mut Vec<f32>, value: f32) {
        history.push(value);
        if history.len() > HISTORY_LEN {
            history.remove(0);
        }
    }
}

impl App for MissionControl {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        let rows = Layout::new(Direction::Vertical, vec![Constraint::Fill(1); 2]).split(area);
        let top = Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 2]).split(rows[0]);
        let bottom =
            Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 2]).split(rows[1]);

        let altitude_inner = Block::new()
            .title("Altitude (m)")
            .theme(&self.theme)
            .render(top[0], buf);
        Sparkline::new(&self.altitude_history, self.theme.primary).render(altitude_inner, buf);

        let velocity_inner = Block::new()
            .title("Velocity (m/s)")
            .theme(&self.theme)
            .render(top[1], buf);
        Sparkline::new(&self.velocity_history, self.theme.primary).render(velocity_inner, buf);

        let signal_inner = Block::new()
            .title("Signal Strength (%)")
            .theme(&self.theme)
            .render(bottom[0], buf);
        Sparkline::new(&self.signal_history, self.theme.accent).render(signal_inner, buf);

        let subsystem_items: Vec<(&str, f32)> = SUBSYSTEM_NAMES
            .iter()
            .zip(self.subsystems.iter())
            .map(|(&name, &health)| (name, health))
            .collect();
        let subsystem_inner = Block::new()
            .title("Subsystem Status")
            .theme(&self.theme)
            .render(bottom[1], buf);
        BarChart::new(&subsystem_items, 100.0, self.theme.secondary).render(subsystem_inner, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, _elapsed: Duration) {
        self.tick_count += 1;
        let base = self.tick_count as u32;
        self.altitude = (self.altitude + scatter(base, ALTITUDE_STEP)).clamp(0.0, 10_000.0);
        Self::push_history(&mut self.altitude_history, self.altitude);
        self.velocity =
            (self.velocity + scatter(base.wrapping_add(1_000), VELOCITY_STEP)).clamp(0.0, 500.0);
        Self::push_history(&mut self.velocity_history, self.velocity);
        self.signal =
            (self.signal + scatter(base.wrapping_add(2_000), SIGNAL_STEP)).clamp(0.0, 100.0);
        Self::push_history(&mut self.signal_history, self.signal);
        for (i, health) in self.subsystems.iter_mut().enumerate() {
            *health = (*health
                + scatter(base.wrapping_add(3_000 + i as u32 * 777), SUBSYSTEM_STEP))
            .clamp(0.0, 100.0);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut app = MissionControl::new();
    run(&mut app)
}
