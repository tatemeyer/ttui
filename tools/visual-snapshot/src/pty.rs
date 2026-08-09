//! Spawns a compiled TTUI example under a real OS pseudo-console
//! (`portable-pty`, ConPTY on Windows) and captures frames of its
//! terminal output as rasterized images, either as a single snapshot
//! (`Session::capture_frame`) or driven through a scripted sequence of
//! key presses and waits (`run_script`).

use crate::keys;
use crate::render::{self, RenderError};
use crate::script::Step;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Errors from spawning or driving a PTY-attached child process.
#[derive(Debug)]
pub enum PtyError {
    /// An I/O failure (writing to the child, running `cargo build`, ...).
    Io(std::io::Error),
    /// A `portable-pty`-reported failure (opening the pty, spawning the
    /// child) — stringified since the underlying error type is `anyhow::Error`.
    Pty(String),
    /// A failure while rasterizing the captured screen.
    Render(RenderError),
}

impl From<std::io::Error> for PtyError {
    fn from(e: std::io::Error) -> Self {
        PtyError::Io(e)
    }
}

impl From<RenderError> for PtyError {
    fn from(e: RenderError) -> Self {
        PtyError::Render(e)
    }
}

/// Builds `cargo build --example <name>` and returns the resulting
/// binary's path (relative to the workspace's shared `target/` dir).
pub fn build_example(name: &str) -> Result<PathBuf, PtyError> {
    let status = StdCommand::new("cargo")
        .args(["build", "--example", name])
        .status()?;
    if !status.success() {
        return Err(PtyError::Pty(format!(
            "cargo build --example {name} failed"
        )));
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/examples");
    path.push(name);
    if cfg!(windows) {
        path.set_extension("exe");
    }
    Ok(path)
}

/// How long `capture_frame` waits for pending output to arrive before
/// parsing whatever has landed in the shared buffer so far.
pub const SETTLE_DELAY: Duration = Duration::from_millis(100);

/// An active PTY-attached child process plus a background thread
/// continuously draining its output into a shared buffer.
pub struct Session {
    parser: vt100::Parser,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    output: Arc<Mutex<Vec<u8>>>,
    // Never read directly — held purely so the pseudo-console (and the
    // reader/writer handles derived from it) stays open for as long as
    // `Session` does. Dropping it early would tear down the pty out
    // from under the background reader thread and `writer`/`parser`.
    #[allow(dead_code)]
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Session {
    /// Spawns `binary` under a new pseudo-console of the given size and
    /// starts a background reader thread.
    pub fn spawn(binary: &Path, rows: u16, cols: u16) -> Result<Session, PtyError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let cmd = CommandBuilder::new(binary);
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Pty(e.to_string()))?;
        // Drop the slave handle once the child is spawned so the master
        // side can observe EOF when the child exits.
        drop(pair.slave);

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Pty(e.to_string()))?;
        let writer = Arc::new(Mutex::new(writer));
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_writer = Arc::clone(&output);
        let dsr_writer = Arc::clone(&writer);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        // Windows' ConPTY (conhost) issues a Device Status
                        // Report cursor-position query (`ESC[6n`) to the
                        // outer terminal shortly after attaching, as part
                        // of its own startup handshake — this is separate
                        // from anything the spawned binary itself writes.
                        // Until something answers with a Cursor Position
                        // Report (`ESC[row;colR`) on the input side,
                        // conhost stalls: it neither forwards the child's
                        // real output nor delivers further input to the
                        // child. Confirmed empirically while building
                        // `run_script` — without this reply, not even a
                        // single raw byte written via `send` ever reached
                        // a spawned child's stdin, regardless of encoding
                        // or wait time. Answering with an arbitrary
                        // position (`1;1`) is sufficient; nothing in this
                        // tool's rendering path depends on cursor state
                        // being accurate. Unix PTYs have no such
                        // handshake, so this is a no-op there — the query
                        // byte sequence never appears in that path.
                        if buf[..n].windows(4).any(|w| w == b"\x1b[6n") {
                            let mut w = dsr_writer.lock().unwrap();
                            let _ = w.write_all(b"\x1b[1;1R");
                            let _ = w.flush();
                        }
                        output_writer.lock().unwrap().extend_from_slice(&buf[..n])
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Session {
            parser: vt100::Parser::new(rows, cols, 0),
            writer,
            output,
            master: pair.master,
            child,
        })
    }

    /// Waits `SETTLE_DELAY`, drains whatever output has arrived since
    /// the last capture into the parser, and rasterizes the current
    /// screen state.
    pub fn capture_frame(&mut self) -> Result<image::RgbaImage, PtyError> {
        thread::sleep(SETTLE_DELAY);
        let pending: Vec<u8> = {
            let mut buf = self.output.lock().unwrap();
            std::mem::take(&mut *buf)
        };
        self.parser.process(&pending);
        Ok(render::render_screen(self.parser.screen())?)
    }

    /// Writes raw bytes into the pseudo-console's input handle.
    pub fn send(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    /// Terminates the child process. Safe to call even after the child
    /// has already exited on its own: verified from the 0.9.0
    /// `portable-pty` source (`src/win/mod.rs`, `WinChild::kill`) that
    /// this project's Windows ConPTY backend already collapses a failed
    /// `TerminateProcess` call to `Ok(())` internally, so there is no
    /// "already exited" failure case for this method to special-case.
    /// The underlying `Result` is still propagated (rather than
    /// blanket-ignored) so a future backend change that *can* report a
    /// genuine termination failure isn't silently discarded here.
    pub fn kill(&mut self) -> Result<(), PtyError> {
        self.child.kill()?;
        Ok(())
    }
}

const KEY_STEP_DISPLAY_DURATION: Duration = Duration::from_millis(150);

/// Spawns `binary`, drives it through `steps` (real key bytes / real
/// wall-clock waits), and returns one rendered frame per step plus an
/// initial frame captured before any step runs.
pub fn run_script(
    binary: &Path,
    rows: u16,
    cols: u16,
    steps: &[Step],
) -> Result<Vec<(image::RgbaImage, Duration)>, PtyError> {
    let mut session = Session::spawn(binary, rows, cols)?;
    let mut frames = Vec::with_capacity(steps.len() + 1);

    frames.push((session.capture_frame()?, Duration::from_millis(0)));

    for step in steps {
        let duration = match step {
            Step::Wait { wait_ms } => {
                std::thread::sleep(Duration::from_millis(*wait_ms));
                Duration::from_millis(*wait_ms)
            }
            Step::Key { key } => {
                let bytes = keys::encode_key(key).map_err(|e| PtyError::Pty(format!("{e:?}")))?;
                session.send(&bytes)?;
                KEY_STEP_DISPLAY_DURATION
            }
        };
        frames.push((session.capture_frame()?, duration));
    }

    session.kill()?;
    Ok(frames)
}

impl Drop for Session {
    /// Ensures the child process doesn't outlive its `Session` even if
    /// the caller never calls `kill()` explicitly (as in ordinary
    /// scope-exit cleanup). Delegates to `kill()`, which is documented
    /// safe to call on an already-exited child; the `Result` can't be
    /// propagated out of `drop`, so it's discarded here specifically
    /// (not the blanket-ignore pattern flagged for the old `kill()`
    /// body — this is the one place ignoring it is unavoidable).
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn echo_key_binary() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../../target/debug/examples/echo_key");
        if cfg!(windows) {
            path.set_extension("exe");
        }
        path
    }

    /// Guards finding #1 from the Task 8 review: cleanup must be a real,
    /// verified effect, not an assumed side effect of `master`'s Drop
    /// impl. Spawns a real child, confirms it's alive, kills it, then
    /// polls `Child::try_wait` (the crate's own liveness check) until it
    /// reports the process gone, bounded so a regression fails fast
    /// instead of hanging.
    #[test]
    fn kill_terminates_a_still_running_child_within_a_bounded_time() {
        let mut session = Session::spawn(&echo_key_binary(), 5, 40).unwrap();
        // The fixture blocks on event::read() until killed or sent
        // input, so it should still be alive immediately after spawn.
        assert!(
            session.child.try_wait().unwrap().is_none(),
            "expected the fixture to still be running before kill()"
        );

        session.kill().unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if session.child.try_wait().unwrap().is_some() {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "child was not reported terminated within 5s of kill()"
            );
            thread::sleep(Duration::from_millis(50));
        }
    }
}
