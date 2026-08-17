//! Fixture binary for the real-TTY test harness: an app that decides to
//! exit from `on_tick` alone, with no input ever sent (ttui#30).
//!
//! `run`'s loop only consulted `should_quit()` after the input arm, so
//! this process would poll forever waiting for a keypress it is never
//! given. It writes its result file *before* returning from `run`, so a
//! hang is distinguishable from an exit that simply failed to report.
use std::env;
use std::fs;
use std::time::Duration;
use ttui::app::{run, App};
use ttui::buffer::LayerStack;
use ttui::layout::Rect;

/// Ticks to observe before asking to quit — more than one, so the test
/// exercises a real timed decision rather than an immediate exit.
const TICKS_UNTIL_QUIT: u32 = 3;

struct TimedExit {
    ticks: u32,
    out_path: String,
    reported: bool,
}

impl App for TimedExit {
    fn view(&self, _area: Rect, _buf: &mut LayerStack) {}

    fn update(&mut self, _event: &crossterm::event::Event) {
        // Deliberately empty: this fixture must never depend on input.
    }

    fn should_quit(&self) -> bool {
        self.ticks >= TICKS_UNTIL_QUIT
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(30))
    }

    fn on_tick(&mut self, _elapsed: Duration) {
        self.ticks += 1;
        if self.ticks >= TICKS_UNTIL_QUIT && !self.reported {
            self.reported = true;
            fs::write(&self.out_path, format!("ticks={}\nquit=true\n", self.ticks)).unwrap();
        }
    }
}

fn main() {
    let out_path = env::args()
        .nth(1)
        .expect("expected an output file path argument");
    let mut app = TimedExit {
        ticks: 0,
        out_path,
        reported: false,
    };
    run(&mut app).unwrap();
}
