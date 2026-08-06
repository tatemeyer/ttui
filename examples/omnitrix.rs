// examples/omnitrix.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{Duration, Instant};
use ttui::app::{run, App};
use ttui::buffer::LayerStack;
use ttui::layout::Rect;
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::{block::Block, list::List, text::Text};

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS

#[derive(Clone, Copy, PartialEq)]
enum DnaSample {
    Brainstorm,
    Fasttrack,
    Upgrade,
}

impl DnaSample {
    const ALL: [DnaSample; 3] = [
        DnaSample::Brainstorm,
        DnaSample::Fasttrack,
        DnaSample::Upgrade,
    ];

    fn name(&self) -> &'static str {
        match self {
            DnaSample::Brainstorm => "Brainstorm",
            DnaSample::Fasttrack => "Fasttrack",
            DnaSample::Upgrade => "Upgrade",
        }
    }
}

enum Screen {
    Faceplate,
    Launched(DnaSample),
}

struct Omnitrix {
    pulse_phase: f32,
    quit: bool,
    last_tick_started: Instant,
    perf_log: std::fs::File,
    selected: usize,
    screen: Screen,
}

impl Omnitrix {
    fn new() -> Self {
        let perf_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open("omnitrix_perf.log")
            .expect("failed to open omnitrix_perf.log");
        Omnitrix {
            pulse_phase: 0.0,
            quit: false,
            last_tick_started: Instant::now(),
            perf_log,
            selected: 0,
            screen: Screen::Faceplate,
        }
    }

    fn theme(&self) -> Theme {
        // Breathing pulse: sine wave brightness between a dim and a
        // bright green, matching the Omnitrix vision doc's "Recharge
        // Pulse" description.
        let brightness = (self.pulse_phase.sin() + 1.0) / 2.0;
        let primary = Color::Rgb {
            r: 0,
            g: (120.0 + brightness * 135.0) as u8,
            b: (32.0 + brightness * 33.0) as u8,
        };
        Theme {
            background: Color::Black,
            primary,
            secondary: Color::DarkGreen,
            tertiary: Color::Red,
            accent: Color::White,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '+',
            },
            border_bold: brightness > 0.6,
        }
    }
}

impl App for Omnitrix {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }
        match self.screen {
            Screen::Faceplate => match k.code {
                KeyCode::Tab => self.selected = (self.selected + 1) % DnaSample::ALL.len(),
                KeyCode::BackTab => {
                    self.selected =
                        (self.selected + DnaSample::ALL.len() - 1) % DnaSample::ALL.len()
                }
                KeyCode::Enter => self.screen = Screen::Launched(DnaSample::ALL[self.selected]),
                _ => {}
            },
            Screen::Launched(_) => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Faceplate;
                }
            }
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        let theme = self.theme();
        let inner = Block::new()
            .title("Omnitrix")
            .theme(&theme)
            .render(area, buf);

        match &self.screen {
            Screen::Faceplate => {
                let names: Vec<String> = DnaSample::ALL
                    .iter()
                    .map(|s| s.name().to_string())
                    .collect();
                List::new(&names, self.selected).render(inner, buf);
            }
            Screen::Launched(sample) => {
                let name_row = Rect {
                    x: inner.x,
                    y: inner.y,
                    width: inner.width,
                    height: 1,
                };
                let placeholder_row = Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: inner.height.saturating_sub(1),
                };
                Text::new(sample.name()).render(name_row, buf);
                Text::new("(not yet built)").render(placeholder_row, buf);
            }
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(TICK_INTERVAL)
    }

    fn on_tick(&mut self, elapsed: Duration) {
        // Measures wall-clock time since the previous tick STARTED,
        // which includes this loop iteration's poll wait plus the
        // PREVIOUS iteration's full render+flush. If Terminal::draw_diff's
        // per-cell execute! pattern (the Rev B spec's open performance
        // risk) is expensive, this value will consistently exceed
        // TICK_INTERVAL by more than the previous frame's render cost
        // should account for. This is a deliberately simple, core-code-free
        // way to get real numbers for a prototype, not a permanent
        // profiling mechanism.
        let now = Instant::now();
        let since_last_tick = now.duration_since(self.last_tick_started);
        self.last_tick_started = now;
        let _ = writeln!(
            self.perf_log,
            "nominal_tick={elapsed:?} actual_time_since_last_tick_start={since_last_tick:?}"
        );

        self.pulse_phase += elapsed.as_secs_f32() * std::f32::consts::PI;
    }
}

fn main() -> std::io::Result<()> {
    let mut app = Omnitrix::new();
    run(&mut app)
}
