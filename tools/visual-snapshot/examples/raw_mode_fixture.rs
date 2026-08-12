//! Fixture binary for the real-TTY test harness: transcribes
//! `enter_and_drop_restores_raw_mode`'s in-process logic (from
//! `src/terminal.rs`) into a standalone process, reporting results via
//! a file (given as the first command-line argument) rather than an
//! in-process assertion.
use std::env;
use std::fs;
use ttui::terminal::Terminal;

fn main() {
    let out_path = env::args()
        .nth(1)
        .expect("expected an output file path argument");
    let before = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
    let during;
    {
        let _term = Terminal::new().unwrap();
        during = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
    }
    let after = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
    fs::write(
        &out_path,
        format!("before={before}\nduring={during}\nafter={after}\n"),
    )
    .unwrap();
}
