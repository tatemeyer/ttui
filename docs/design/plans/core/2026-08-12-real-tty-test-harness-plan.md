# Real-TTY Test Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the "permanently manual" gap for `src/terminal.rs`'s two real-TTY-only tests by extending `tools/visual-snapshot`'s already-proven PTY infrastructure with fixture binaries and automated integration tests.

**Architecture:** Two new `Session` methods (`spawn_with_args`, `wait_for_exit`) generalize the existing PTY-spawn mechanism to fixtures that take arguments and to plain exit-code/file-based verification (rather than screen rendering). Two new fixture binaries transcribe the existing ignored tests' logic into standalone processes, reporting results via a file rather than screen text (avoiding an alternate-screen-buffer pitfall). Two new integration tests spawn those fixtures and assert on the file contents.

**Tech Stack:** Rust, `portable-pty` (already a `tools/visual-snapshot` dependency), `tempfile` (already a dev-dependency there).

## Global Constraints

- **Everything in this plan is `coding`-tagged with full TDD mandatory** — no exemption, this is test infrastructure itself.
- **The existing `#[ignore]`d tests in `src/terminal.rs` are untouched** — they remain exactly as they are, a legitimate cheap manual fallback, not replaced.
- **`portable-pty` does not become a dependency of the root `ttui` library crate** — it stays confined to `tools/visual-snapshot`.
- **No CI workflow changes** — the new tests are ordinary `cargo test`s inside the existing `tools/visual-snapshot` package, already part of the workspace `test` job.
- **Results are reported via a file, not parsed screen text** — `Terminal::new()` swaps to the alternate screen buffer, which would strand anything printed mid-lifetime on a buffer that gets discarded when the guard drops.

---

### Task 1: `Session` additions — `spawn_with_args`, `wait_for_exit`

**Files:**
- Modify: `tools/visual-snapshot/src/pty.rs`
- Create: `tools/visual-snapshot/examples/echo_args.rs`
- Modify: `tools/visual-snapshot/Cargo.toml`

**Interfaces:**
- Produces: `Session::spawn_with_args(binary: &Path, rows: u16, cols: u16, args: &[&str]) -> Result<Session, PtyError>`, `Session::wait_for_exit(&mut self, timeout: Duration) -> Option<portable_pty::ExitStatus>` (returns `Some(status)` if the child exited within `timeout`, `None` if the timeout elapsed first) — both consumed by Task 3. Note: `wait_for_exit` returns `Option<ExitStatus>`, not a bare `bool`, so callers that want the more informative exit-status/success check can have it; a caller that only cares whether it exited in time uses `.is_some()`.

- [ ] **Step 1: Write the failing tests**

Add to `tools/visual-snapshot/src/pty.rs`'s existing `#[cfg(test)] mod tests` block:

```rust
    fn echo_args_binary() -> PathBuf {
        let mut path = examples_dir();
        path.push("echo_args");
        if cfg!(windows) {
            path.set_extension("exe");
        }
        path
    }

    #[test]
    fn spawn_with_args_threads_arguments_to_the_child() {
        let mut session =
            Session::spawn_with_args(&echo_args_binary(), 5, 40, &["hello", "world"]).unwrap();
        let status = session.wait_for_exit(Duration::from_secs(5));
        assert!(status.is_some(), "fixture did not exit in time");
        assert!(
            status.unwrap().success(),
            "fixture reported unexpected args (exited non-zero)"
        );
    }

    #[test]
    fn spawn_with_args_with_empty_args_behaves_like_spawn() {
        // spawn itself is just spawn_with_args(binary, rows, cols, &[]) —
        // confirms an empty args slice doesn't change ordinary behavior.
        let mut session = Session::spawn_with_args(&echo_key_binary(), 5, 40, &[]).unwrap();
        let frame = session.capture_frame().unwrap();
        assert_eq!(frame.width(), 40 * 16);
        assert_eq!(frame.height(), 5 * 16);
    }

    #[test]
    fn wait_for_exit_returns_some_for_a_process_that_exits_on_its_own() {
        let mut session = Session::spawn(&echo_args_binary(), 5, 40).unwrap();
        let status = session.wait_for_exit(Duration::from_secs(5));
        assert!(status.is_some());
    }

    #[test]
    fn wait_for_exit_returns_none_when_the_timeout_elapses_first() {
        // echo_key blocks on event::read() until killed or sent input,
        // so it never exits on its own — a short timeout should elapse.
        let mut session = Session::spawn(&echo_key_binary(), 5, 40).unwrap();
        let status = session.wait_for_exit(Duration::from_millis(200));
        assert!(
            status.is_none(),
            "expected the timeout to elapse since echo_key never exits on its own"
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package visual-snapshot --lib pty::`
Expected: FAIL to compile — `spawn_with_args`, `wait_for_exit`, and the `echo_args` binary don't exist yet.

- [ ] **Step 3: Add the `echo_args` fixture**

Create `tools/visual-snapshot/examples/echo_args.rs`:

```rust
//! Minimal fixture binary for `Session::spawn_with_args` tests: exits
//! 0 if invoked with exactly the args `["hello", "world"]`, exits 1
//! otherwise.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == vec!["hello".to_string(), "world".to_string()] {
        std::process::exit(0);
    }
    std::process::exit(1);
}
```

Add a matching `[[example]]` entry to `tools/visual-snapshot/Cargo.toml`, next to the existing entries:

```toml
[[example]]
name = "echo_args"
```

- [ ] **Step 4: Refactor `spawn` and add `spawn_with_args`**

In `tools/visual-snapshot/src/pty.rs`, `Session::spawn`'s current signature and first few lines:

```rust
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
```

Change `spawn` to delegate to a new `spawn_with_args`, and give `spawn_with_args` the args-threading logic:

```rust
    /// Spawns `binary` under a new pseudo-console of the given size
    /// and starts a background reader thread. Equivalent to
    /// `spawn_with_args(binary, rows, cols, &[])`.
    pub fn spawn(binary: &Path, rows: u16, cols: u16) -> Result<Session, PtyError> {
        Self::spawn_with_args(binary, rows, cols, &[])
    }

    /// Like `spawn`, but also passes `args` to the child's command
    /// line — needed by fixtures that take arguments (e.g. an output
    /// file path).
    pub fn spawn_with_args(
        binary: &Path,
        rows: u16,
        cols: u16,
        args: &[&str],
    ) -> Result<Session, PtyError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let mut cmd = CommandBuilder::new(binary);
        for a in args {
            cmd.arg(a);
        }
```

The rest of the method body (everything from `let child = pair.slave.spawn_command(cmd)...` onward) stays exactly as it currently is in `spawn` — only the signature, the delegation, and the `cmd` construction change.

- [ ] **Step 5: Add `wait_for_exit`**

Add this method to `impl Session` in `tools/visual-snapshot/src/pty.rs`, near `kill`:

```rust
    /// Polls `self.child.try_wait()` until it reports the process
    /// gone or `timeout` elapses. Returns `Some(status)` if the child
    /// exited within `timeout`, `None` if the timeout elapsed first.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Option<portable_pty::ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --package visual-snapshot --lib pty::`
Expected: all 4 new tests PASS, plus all existing `pty::` tests (the private test module's own existing tests) still pass.

- [ ] **Step 7: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 8: Full workspace test**

Run: `cargo test`
Expected: full workspace suite green, including `tests/pty_roundtrip.rs`'s existing tests, unaffected by this refactor.

- [ ] **Step 9: Commit**

```bash
git add tools/visual-snapshot/src/pty.rs tools/visual-snapshot/examples/echo_args.rs \
        tools/visual-snapshot/Cargo.toml
git commit -m "feat(visual-snapshot): add Session::spawn_with_args and wait_for_exit

Generalizes the existing PTY-spawn mechanism to fixtures that take
arguments and to plain exit-code-based verification, needed by the
upcoming raw-mode/panic-hook real-TTY integration tests."
```

---

### Task 2: Two fixture binaries

**Files:**
- Create: `tools/visual-snapshot/examples/raw_mode_fixture.rs`
- Create: `tools/visual-snapshot/examples/panic_hook_fixture.rs`
- Modify: `tools/visual-snapshot/Cargo.toml`

**Interfaces:**
- Consumes: `ttui::terminal::{Terminal, install_panic_hook}` (the root crate's own public API, already stable). `tools/visual-snapshot` needs `ttui` (the root crate) as a dependency for these fixtures to compile — check `tools/visual-snapshot/Cargo.toml`'s existing `[dependencies]`/`[dev-dependencies]`; if `ttui` isn't already listed there, add it as a path dependency (`ttui = { path = "../.." }`) so these examples can `use ttui::terminal::...`.
- Produces: two fixture binaries — consumed by Task 3's integration tests.

- [ ] **Step 1: Confirm (or add) the `ttui` path dependency**

Check `tools/visual-snapshot/Cargo.toml`'s `[dependencies]` section. If `ttui` is not already listed, add:

```toml
ttui = { path = "../.." }
```

(If it's already present — e.g. because an earlier Arc added it for a different reason — skip this step and note that in your report.)

- [ ] **Step 2: Add `raw_mode_fixture.rs`**

Create `tools/visual-snapshot/examples/raw_mode_fixture.rs`:

```rust
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
```

- [ ] **Step 3: Add `panic_hook_fixture.rs`**

Create `tools/visual-snapshot/examples/panic_hook_fixture.rs`:

```rust
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
    let result = std::panic::catch_unwind(|| {
        panic!("simulated crash");
    });
    let raw_after_panic = crossterm::terminal::is_raw_mode_enabled().unwrap_or(true);
    fs::write(
        &out_path,
        format!(
            "panicked={}\nraw_after_panic={raw_after_panic}\n",
            result.is_err()
        ),
    )
    .unwrap();
}
```

- [ ] **Step 4: Register both examples**

Add two `[[example]]` entries to `tools/visual-snapshot/Cargo.toml`:

```toml
[[example]]
name = "raw_mode_fixture"

[[example]]
name = "panic_hook_fixture"
```

- [ ] **Step 5: Build**

Run: `cargo build --package visual-snapshot --example raw_mode_fixture` — succeeds.
Run: `cargo build --package visual-snapshot --example panic_hook_fixture` — succeeds.

- [ ] **Step 6: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 7: Commit**

```bash
git add tools/visual-snapshot/examples/raw_mode_fixture.rs \
        tools/visual-snapshot/examples/panic_hook_fixture.rs \
        tools/visual-snapshot/Cargo.toml
git commit -m "feat(visual-snapshot): add raw-mode and panic-hook real-TTY fixtures

Faithful transcriptions of src/terminal.rs's two existing #[ignore]d
tests' logic into standalone processes — a custom panic hook set via
std::panic::set_hook still fires when the panic is caught by
catch_unwind, so panic_hook_fixture is a correct transcription, not a
new assumption."
```

---

### Task 3: Two integration tests

**Files:**
- Create: `tools/visual-snapshot/tests/raw_mode_roundtrip.rs`

**Interfaces:**
- Consumes: `Session::spawn_with_args`, `Session::wait_for_exit` (Task 1), the two fixture binaries (Task 2).
- Produces: nothing consumed by later tasks (Task 5 is verification-only).

- [ ] **Step 1: Write the tests**

Create `tools/visual-snapshot/tests/raw_mode_roundtrip.rs`:

```rust
use std::path::PathBuf;
use std::time::Duration;
use tempfile::tempdir;
use visual_snapshot::pty::{examples_dir, Session};

fn fixture_binary(name: &str) -> PathBuf {
    let mut path = examples_dir();
    path.push(name);
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

#[test]
fn raw_mode_enters_during_the_terminal_guards_lifetime_and_exits_after_drop() {
    let dir = tempdir().unwrap();
    let out_path = dir.path().join("result.txt");
    let mut session = Session::spawn_with_args(
        &fixture_binary("raw_mode_fixture"),
        5,
        40,
        &[out_path.to_str().unwrap()],
    )
    .unwrap();
    let status = session.wait_for_exit(Duration::from_secs(5));
    assert!(status.is_some(), "fixture did not exit in time");
    let contents = std::fs::read_to_string(&out_path).unwrap();
    assert!(contents.contains("before=false"), "{contents}");
    assert!(contents.contains("during=true"), "{contents}");
    assert!(contents.contains("after=false"), "{contents}");
}

#[test]
fn panic_hook_disables_raw_mode_before_the_panic_propagates() {
    let dir = tempdir().unwrap();
    let out_path = dir.path().join("result.txt");
    let mut session = Session::spawn_with_args(
        &fixture_binary("panic_hook_fixture"),
        5,
        40,
        &[out_path.to_str().unwrap()],
    )
    .unwrap();
    let status = session.wait_for_exit(Duration::from_secs(5));
    assert!(status.is_some(), "fixture did not exit in time");
    let contents = std::fs::read_to_string(&out_path).unwrap();
    assert!(contents.contains("panicked=true"), "{contents}");
    assert!(contents.contains("raw_after_panic=false"), "{contents}");
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test --package visual-snapshot --test raw_mode_roundtrip`
Expected: both tests PASS — no `--ignored` flag needed. This is the actual gap-closing moment: these are ordinary `cargo test`s exercising the same behavior `src/terminal.rs`'s manual-only tests check, but automatically.

If either test fails, debug before proceeding — do not weaken an assertion to make it pass. A failure here means either the fixture's transcription of the original test's logic has a bug, or `Terminal`'s raw-mode-enter/exit or panic-hook behavior doesn't work under a `portable-pty`-hosted console the way it does directly against a real terminal, which would itself be a genuine, important finding to report rather than paper over.

- [ ] **Step 3: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 4: Full workspace test**

Run: `cargo test`
Expected: full workspace suite green, including Task 1's new `pty::` tests and these two new integration tests.

- [ ] **Step 5: Commit**

```bash
git add tools/visual-snapshot/tests/raw_mode_roundtrip.rs
git commit -m "test(visual-snapshot): add automated real-TTY raw-mode/panic-hook tests

Closes the gap development-conventions.md documents as 'permanently
manual' — these run on every ordinary cargo test, no --ignored flag,
no CI workflow change, via a spawned child's synthetic real console
rather than the test process's own (absent) one."
```

---

### Task 4: Documentation update

**Files:**
- Modify: `.claude/rules/development-conventions.md`

**Interfaces:** none.

- [ ] **Step 1: Update the "Real-TTY tests" section**

Find the "Real-TTY tests" section in `.claude/rules/development-conventions.md` (currently states these tests are "permanently manual — not 'for now.'" and that a self-hosted runner was "considered and rejected"). Add a new paragraph immediately after the existing content (do not delete or rewrite the existing paragraphs — this is additive):

```markdown
**Update (2026-08-12):** `src/terminal.rs`'s two `#[ignore]`d tests
remain, unchanged, as a legitimate cheap manual fallback — but they
are no longer the *only* coverage for this behavior.
`tools/visual-snapshot/tests/raw_mode_roundtrip.rs` now exercises the
identical logic automatically, via a spawned child's synthetic real
console (`portable-pty`/ConPTY) rather than the test process's own
(absent) one, and runs on every `cargo test` with no CI workflow
change needed. The self-hosted-runner rejection above stands, and for
a more precise reason than originally stated: the runner was never
actually the blocker — a spawned child's synthetic console was
sufficient all along, and ordinary hosted runners already support it,
proven by this exact mechanism running in CI on every PR since
`tools/visual-snapshot` first landed.
```

- [ ] **Step 2: Commit**

```bash
git add .claude/rules/development-conventions.md
git commit -m "docs(core): update Real-TTY tests section to reflect the new harness

Additive — the existing ignored-tests-are-manual guidance stays, this
just documents that automated coverage now also exists via
tools/visual-snapshot's PTY harness."
```

---

### Task 5: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: full workspace suite green — includes all of Task 1's `pty::` tests, Task 3's `raw_mode_roundtrip.rs` tests, and everything else unchanged.

- [ ] **Step 4: Confirm no `--ignored` flag was needed**

Run: `cargo test --package visual-snapshot --test raw_mode_roundtrip -v` (or equivalent verbose flag) one more time, standalone, and note in your report that both tests ran and passed with the default `cargo test` invocation — no `--ignored`. This is the concrete, checkable proof that the gap described in this plan's Goal is closed.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo test` — full suite green, including all new `pty::`/`raw_mode_roundtrip` tests.
- [ ] The PR's Verification section states plainly that `cargo test` (no `--ignored`) now exercises real raw-mode enter/exit and panic-hook behavior end-to-end, and names the two new test functions.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree` (per the documented squash-merge resolution: verify via `gh pr view --json state,mergedAt,mergeCommit`, then retry with `discard_changes: true` if the tool's own ancestry check false-positives).
