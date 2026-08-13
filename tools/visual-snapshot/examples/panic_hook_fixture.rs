//! Fixture binary for the real-TTY test harness: transcribes
//! `panic_hook_disables_raw_mode_before_unwinding`'s in-process logic
//! (from `src/terminal.rs`) into a standalone process, reporting
//! results via a file (given as the first command-line argument).
use std::env;
use std::fs;
use ttui::terminal::install_panic_hook;

fn main() {
    let out_path = env::args()
        .nth(1)
        .expect("expected an output file path argument");
    install_panic_hook();
    crossterm::terminal::enable_raw_mode().unwrap();
    let raw_before_panic = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
    let result = std::panic::catch_unwind(|| {
        panic!("simulated crash");
    });
    let raw_after_panic = crossterm::terminal::is_raw_mode_enabled().unwrap_or(true);
    fs::write(
        &out_path,
        format!(
            "panicked={}\nraw_before_panic={raw_before_panic}\nraw_after_panic={raw_after_panic}\n",
            result.is_err()
        ),
    )
    .unwrap();
}
