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
use ttui::widgets::{block::Block, text::Text};

const TICK_INTERVAL: Duration = Duration::from_millis(33); // ~30 FPS

struct Omnitrix {
    pulse_phase: f32,
    quit: bool,
    last_tick_started: Instant,
    perf_log: std::fs::File,
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
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        let theme = self.theme();
        let inner = Block::new()
            .title("Omnitrix")
            .theme(&theme)
            .render(area, buf);
        Text::new("The Omnitrix breathes... press q to quit").render(inner, buf);
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
