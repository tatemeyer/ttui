use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use crossterm::style::Color;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::{CellStyle, LayerStack};
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::particles::{Particle, ParticleSystem};
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{analog_toggle::AnalogToggle, block::Block, dial::Dial, text::Text};

const LAUNCH_SPARK_COUNT: usize = 8;
const LAUNCH_SPARK_LIFETIME_MS: u64 = 400;
const TOGGLE_LABELS: [&str; 3] = ["POWER", "SHIELDS", "COMMS"];

fn control_panel_theme() -> Theme {
    Theme {
        background: Color::Rgb {
            r: 10,
            g: 10,
            b: 10,
        },
        primary: Color::Rgb {
            r: 0,
            g: 255,
            b: 120,
        },
        secondary: Color::Rgb {
            r: 200,
            g: 200,
            b: 200,
        },
        tertiary: Color::Rgb {
            r: 255,
            g: 40,
            b: 40,
        },
        accent: Color::Rgb {
            r: 255,
            g: 200,
            b: 0,
        },
        primary_end: None,
        border: BorderSet::default(),
        border_style: CellStyle::default(),
        border_thick: false,
    }
}

struct ControlPanel {
    theme: Theme,
    toggles: [bool; 3],
    dial_items: Vec<String>,
    dial_selected: usize,
    particles: ParticleSystem,
    button_area: std::cell::Cell<Rect>,
    toggle_areas: std::cell::Cell<[Rect; 3]>,
    dial_area: std::cell::Cell<Rect>,
    quit: bool,
}

impl ControlPanel {
    fn new() -> Self {
        let zero_rect = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        ControlPanel {
            theme: control_panel_theme(),
            toggles: [false, false, false],
            dial_items: vec![
                "STANDBY".into(),
                "PATROL".into(),
                "COMBAT".into(),
                "STEALTH".into(),
            ],
            dial_selected: 0,
            particles: ParticleSystem::new(),
            button_area: std::cell::Cell::new(zero_rect),
            toggle_areas: std::cell::Cell::new([zero_rect; 3]),
            dial_area: std::cell::Cell::new(zero_rect),
            quit: false,
        }
    }

    fn spawn_launch_burst(&mut self, cx: f32, cy: f32) {
        for i in 0..LAUNCH_SPARK_COUNT {
            let angle = i as f32 * std::f32::consts::TAU / LAUNCH_SPARK_COUNT as f32;
            self.particles.spawn(Particle {
                x: cx,
                y: cy,
                vx: angle.cos() * 8.0,
                vy: angle.sin() * 4.0,
                symbol: '*',
                color: self.theme.accent,
                lifetime: Duration::from_millis(LAUNCH_SPARK_LIFETIME_MS),
                age: Duration::ZERO,
            });
        }
    }
}

impl App for ControlPanel {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char('q') => {
                self.quit = true;
            }
            Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => {
                let button = self.button_area.get();
                if button.contains(m.column, m.row) {
                    let cx = button.x as f32 + button.width as f32 / 2.0;
                    let cy = button.y as f32 + button.height as f32 / 2.0;
                    self.spawn_launch_burst(cx, cy);
                    return;
                }
                for (i, area) in self.toggle_areas.get().iter().enumerate() {
                    if area.contains(m.column, m.row) {
                        self.toggles[i] = !self.toggles[i];
                        return;
                    }
                }
                let dial = self.dial_area.get();
                if dial.contains(m.column, m.row) {
                    self.dial_selected = (self.dial_selected + 1) % self.dial_items.len();
                }
            }
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        let rows = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Fill(1),
            ],
        )
        .split(area);

        let button_inner = Block::new()
            .title("LAUNCH")
            .theme(&self.theme)
            .render(rows[0], buf);
        self.button_area.set(rows[0]);
        Text::new("Click to launch").render(button_inner, buf);

        let toggle_cols =
            Layout::new(Direction::Horizontal, vec![Constraint::Fill(1); 3]).split(rows[1]);
        let mut toggle_areas = [Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }; 3];
        for (i, col) in toggle_cols.iter().enumerate() {
            let inner = Block::new()
                .title(TOGGLE_LABELS[i])
                .theme(&self.theme)
                .render(*col, buf);
            toggle_areas[i] = *col;
            AnalogToggle::new(self.toggles[i]).render(inner, buf);
        }
        self.toggle_areas.set(toggle_areas);

        let dial_inner = Block::new()
            .title("MODE")
            .theme(&self.theme)
            .render(rows[2], buf);
        self.dial_area.set(rows[2]);
        Dial::new(&self.dial_items, self.dial_selected).render(dial_inner, buf);

        let overlay = buf.push_layer();
        self.particles.render(overlay);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(33))
    }

    fn on_tick(&mut self, elapsed: Duration) {
        self.particles.update(elapsed);
    }
}

fn main() -> std::io::Result<()> {
    let mut app = ControlPanel::new();
    run(&mut app)
}
