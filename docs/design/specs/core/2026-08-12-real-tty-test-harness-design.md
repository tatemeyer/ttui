# Real-TTY Test Harness — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-12
**Relationship to prior specs:** a fresh Arc prompted by investigating
whether an agent could run tests against a real terminal. Builds
directly on `tools/visual-snapshot`'s existing PTY infrastructure
(`src/pty.rs`'s `Session`, `portable-pty`/ConPTY-backed, already
proven and already exercised by `tests/pty_roundtrip.rs` on every CI
run) — this Arc doesn't invent new PTY plumbing, it extends the
existing, working mechanism to reach two specific `#[ignore]`d tests
in `src/terminal.rs` that `.claude/rules/development-conventions.md`'s
"Real-TTY tests" section currently documents as "permanently manual."

## Problem

`src/terminal.rs` has two tests marked `#[ignore = "requires a real
terminal (TTY)"]`: `enter_and_drop_restores_raw_mode` and
`panic_hook_disables_raw_mode_before_unwinding`. They run in-process,
checking `crossterm::terminal::is_raw_mode_enabled()` against whatever
TTY the *test runner itself* has — which is nothing, in CI or this
sandboxed dev environment, so they're skipped by `cargo test`'s
default `#[ignore]` exclusion and only ever run manually, per PR, by
whoever remembers to. `development-conventions.md` explicitly
considered and rejected a self-hosted runner with real TTY access to
close this gap, reasoning the infrastructure burden wasn't justified.

That rejection is almost certainly still the right call — but for a
narrower reason than "no solution exists." `tools/visual-snapshot`
already proves a *different*, much cheaper mechanism: a **spawned
child** process gets a genuine synthetic console via `portable-pty`
(real ConPTY on Windows, real PTYs on Unix) without the *host* needing
any special TTY access at all — already running successfully on
whatever runner this project's existing `test` CI job uses, every PR,
all session. This spec closes the gap using that mechanism instead of
the one already (correctly) rejected.

## Scope

**Tag: `coding`, TDD mandatory** — this is real automated test
coverage; every new fixture/assertion is itself something to verify
works, not exempt example code.

Three slices, in dependency order:

1. **`Session` additions** (`tools/visual-snapshot/src/pty.rs`) —
   `spawn_with_args` and `wait_for_exit`.
2. **Two fixture binaries** (`tools/visual-snapshot/examples/`) —
   depends on 1 only insofar as they're the things Slice 3 spawns.
3. **Two integration tests** (`tools/visual-snapshot/tests/`) —
   depends on 1-2.

Plus a documentation update to `development-conventions.md`'s "Real-TTY
tests" section, reflecting the new state.

## Design

### Why a file-based sentinel, not screen-text parsing

`tools/visual-snapshot`'s existing `Session` reads a spawned child's
output through a `vt100::Parser`, exposing rendered *screen* content —
built for capturing what a TUI app visually draws. `Terminal::new()`
enters the alternate screen buffer as part of its normal setup; any
plain `println!` a fixture emits *while* a `Terminal` guard is alive
would land on that alternate buffer, then get discarded the moment
`Drop` restores the primary buffer — stranding exactly the "during"
checkpoint this design needs, before the fixture even finishes. Rather
than fight that, both fixtures below write their sentinel results to
a plain file the outer test also has a handle to, sidestepping
PTY/terminal-buffer semantics entirely for values that were never
visual to begin with (three booleans).

### Slice 1: `Session` additions

`tools/visual-snapshot/src/pty.rs` gains two small methods:

```rust
impl Session {
    /// Like `spawn`, but also passes `args` to the child's command
    /// line — needed by fixtures that take an output-file path as an
    /// argument.
    pub fn spawn_with_args(
        binary: &Path,
        rows: u16,
        cols: u16,
        args: &[&str],
    ) -> Result<Session, PtyError> {
        // Same body as `spawn`, except:
        // let mut cmd = CommandBuilder::new(binary);
        // for a in args { cmd.arg(a); }
        // ...
    }

    /// Polls `self.child.try_wait()` until it reports the process
    /// gone or `timeout` elapses. Returns whether it exited in time.
    /// Generalizes the bounded-wait loop `kill_terminates_a_still_
    /// running_child_within_a_bounded_time` (this file's own private
    /// test module) already uses inline, as a public method usable
    /// from a separate `tests/` integration file, which can't reach
    /// `Session.child` directly (private field).
    pub fn wait_for_exit(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.child.try_wait().ok().flatten().is_some() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}
```

`spawn`'s existing body is refactored so both constructors share the
same pty-setup/reader-thread logic, `spawn` simply calling
`spawn_with_args(binary, rows, cols, &[])`.

### Slice 2: Two fixture binaries

`tools/visual-snapshot/examples/raw_mode_fixture.rs` — a faithful
transcription of `enter_and_drop_restores_raw_mode`'s existing
in-process logic into a standalone binary, reporting via a file
instead of an assertion:

```rust
use std::env;
use std::fs;
use ttui::terminal::Terminal;

fn main() {
    let out_path = env::args().nth(1).expect("expected an output file path argument");
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

`tools/visual-snapshot/examples/panic_hook_fixture.rs` — likewise
transcribing `panic_hook_disables_raw_mode_before_unwinding`'s logic:

```rust
use std::env;
use std::fs;
use ttui::terminal::install_panic_hook;

fn main() {
    let out_path = env::args().nth(1).expect("expected an output file path argument");
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

A custom panic hook set via `std::panic::set_hook` still fires when
the panic is caught by `catch_unwind` — the hook runs as part of the
panicking mechanism itself, before `catch_unwind` intercepts the
unwind, so this is a correct, faithful transcription of the original
test's already-proven logic, not a new assumption.

Both fixtures are added to `tools/visual-snapshot`'s `[[example]]`
list in `Cargo.toml`, next to the three that already exist
(`echo_key`, `delayed_draw`, `delayed_key_response`).

### Slice 3: Two integration tests

New file `tools/visual-snapshot/tests/raw_mode_roundtrip.rs`, sibling
to the existing `pty_roundtrip.rs`:

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
    assert!(
        session.wait_for_exit(Duration::from_secs(5)),
        "fixture did not exit in time"
    );
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
    assert!(
        session.wait_for_exit(Duration::from_secs(5)),
        "fixture did not exit in time"
    );
    let contents = std::fs::read_to_string(&out_path).unwrap();
    assert!(contents.contains("panicked=true"), "{contents}");
    assert!(contents.contains("raw_after_panic=false"), "{contents}");
}
```

These run as ordinary `cargo test`s — no `--ignored`, no CI workflow
change. They're part of the same workspace `test` job that already
runs `tools/visual-snapshot`'s existing `pty_roundtrip.rs` suite on
every PR.

### Documentation update

`.claude/rules/development-conventions.md`'s "Real-TTY tests" section
gets a new paragraph describing this: `src/terminal.rs`'s two
`#[ignore]`d tests remain, unchanged, as a legitimate cheap manual
fallback — but they are no longer the *only* coverage for this
behavior. `tools/visual-snapshot/tests/raw_mode_roundtrip.rs` now
exercises the identical logic automatically, via a spawned-child real
console rather than the test process's own (absent) one, and runs on
every `cargo test` with no CI changes needed. The self-hosted-runner
rejection stands, correctly, for a different reason than originally
stated: the runner was never the actual blocker — a spawned child's
synthetic console was sufficient all along, and ordinary hosted
runners already support it.

## Non-goals

- **A general structured-assertion API for testing arbitrary example
  apps' interactive behavior.** This closes exactly the one gap
  `development-conventions.md` already named (raw-mode enter/exit,
  panic-hook behavior) — not a new general-purpose test framework.
- **Removing or modifying the existing `#[ignore]`d tests in
  `src/terminal.rs`.** They stay exactly as they are.
- **Adding `portable-pty` as a dependency of the root `ttui` library
  crate.** It stays confined to `tools/visual-snapshot`, which already
  depends on it — the published framework crate's dependency footprint
  is unaffected.
- **Screen-text-based verification for this specific gap.** File-based
  sentinels are used instead, for the alternate-screen-buffer reason
  explained above — `Session`'s existing `vt100`-based screen-text
  reading is untouched and still used exactly as before for visual
  captures.

## Testing

Per `.claude/rules/development-conventions.md`, this is `coding`-tagged
with full TDD mandatory:

- `spawn_with_args` behaves identically to `spawn` when called with an
  empty args slice (no regression to existing callers/tests).
- `wait_for_exit` returns `true` promptly for a process that exits on
  its own; returns `false` (not panicking or hanging) if the timeout
  elapses first — provable with a fixture that sleeps past a short
  timeout, or by reusing an existing long-lived fixture like
  `echo_key` with a deliberately short timeout.
- The two new integration tests themselves *are* the coverage for the
  raw-mode/panic-hook gap — no separate unit test needed beyond them,
  since they exercise the real, complete, end-to-end scenario the
  original `#[ignore]`d tests could only approximate in-process.

## Critical files

- `tools/visual-snapshot/src/pty.rs` — `spawn_with_args`,
  `wait_for_exit`.
- `tools/visual-snapshot/examples/raw_mode_fixture.rs`,
  `tools/visual-snapshot/examples/panic_hook_fixture.rs` — new
  fixtures.
- `tools/visual-snapshot/Cargo.toml` — two new `[[example]]` entries.
- `tools/visual-snapshot/tests/raw_mode_roundtrip.rs` — new
  integration test file.
- `.claude/rules/development-conventions.md` — "Real-TTY tests"
  section update.

## Verification

- `cargo build --all-targets` / `cargo clippy --all-targets -- -D
  warnings` / `cargo fmt --check` — clean.
- `cargo test` — the two new integration tests pass (genuinely, not
  skipped — no `--ignored` needed), full existing suite unchanged
  elsewhere, including `pty_roundtrip.rs`'s existing tests still green.
- Manually confirm (once, during implementation) that `cargo test`
  run with **no** `--ignored` flag on an ordinary developer machine —
  or in CI — now genuinely exercises real raw-mode enter/exit and
  panic-hook behavior end-to-end, closing the gap
  `development-conventions.md` currently documents as permanently
  manual.
