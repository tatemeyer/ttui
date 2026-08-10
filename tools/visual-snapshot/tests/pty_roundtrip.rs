use std::path::PathBuf;
use visual_snapshot::pty::Session;

fn echo_key_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/examples/echo_key");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

fn delayed_draw_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/examples/delayed_draw");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

#[test]
fn spawning_and_capturing_one_frame_shows_the_process_alive() {
    let mut session = Session::spawn(&echo_key_binary(), 5, 40).unwrap();
    // No input sent yet — the fixture is blocked on event::read(), so
    // the initial frame should just be a blank screen at the right size.
    let frame = session.capture_frame().unwrap();
    assert_eq!(frame.width(), 40 * 16);
    assert_eq!(frame.height(), 5 * 16);
}

use visual_snapshot::pty::run_script;
use visual_snapshot::script::Step;

#[test]
fn a_key_step_actually_reaches_the_child_process() {
    let steps = vec![
        Step::Key {
            key: "a".to_string(),
        },
        Step::Wait { wait_ms: 16 },
        Step::Key {
            key: "Esc".to_string(),
        },
    ];

    let frames = run_script(&echo_key_binary(), 5, 40, &steps).unwrap();

    // Initial frame + one per step.
    assert_eq!(frames.len(), 4);
    // The frame captured after sending "a" should show the fixture's
    // echoed `KeyCode::Char('a')` debug text somewhere on screen —
    // checked indirectly via a non-blank pixel outside the top-left
    // origin, since asserting exact glyph pixels here would duplicate
    // render.rs's own tests.
    let after_a = &frames[1].0;
    let any_non_background = after_a.pixels().any(|p| *p != image::Rgba([0, 0, 0, 255]));
    assert!(
        any_non_background,
        "expected the echoed key text to draw something"
    );
}

#[test]
fn frame_durations_match_each_steps_own_timing() {
    let steps = vec![
        Step::Wait { wait_ms: 250 },
        Step::Key {
            key: "Esc".to_string(),
        },
    ];

    let frames = run_script(&echo_key_binary(), 5, 40, &steps).unwrap();

    assert_eq!(frames[0].1, std::time::Duration::from_millis(0)); // initial frame
    assert_eq!(frames[1].1, std::time::Duration::from_millis(250)); // Wait step
    assert_eq!(frames[2].1, std::time::Duration::from_millis(150)); // Key step, fixed duration
}

/// Guards the Task 12 flakiness fix: the old `capture_frame` slept a
/// single fixed 100ms `SETTLE_DELAY` and then snapshotted whatever was
/// in the buffer, regardless of whether the child had actually drawn
/// anything yet. Real TUI examples (unlike the trivial fixtures above)
/// can take meaningfully longer than 100ms to reach their first draw,
/// which produced a real, reproduced blank-frame failure against the
/// `tardis` example (see the Task 12 flakiness fix report). This test
/// exercises that failure mode deterministically: `delayed_draw` sleeps
/// 500ms — 5x the old fixed delay — before writing anything. A single
/// `capture_frame()` call (no test-side retry loop, exactly how
/// `run_script` uses it) must still see that output, proving
/// `capture_frame` now waits for real content instead of giving up
/// early.
#[test]
fn capture_frame_waits_past_the_old_fixed_settle_delay_for_a_slow_first_draw() {
    let mut session = Session::spawn(&delayed_draw_binary(), 5, 40).unwrap();
    let frame = session.capture_frame().unwrap();
    let any_non_background = frame.pixels().any(|p| *p != image::Rgba([0, 0, 0, 255]));
    assert!(
        any_non_background,
        "expected the delayed draw to have been captured, not missed"
    );
}

/// Companion to the test above: proves the fix doesn't sacrifice the
/// common case to fix the slow one. `echo_key` reacts to a keypress
/// almost immediately, so `capture_frame` should quiesce and return
/// well before `MAX_SETTLE_WAIT` (2s) — asserted generously (under 1s)
/// to stay robust against ordinary scheduling jitter while still
/// clearly distinguishing "quiesced quickly" from "hit the max bound".
#[test]
fn capture_frame_stays_fast_when_output_arrives_quickly() {
    let mut session = Session::spawn(&echo_key_binary(), 5, 40).unwrap();
    let _ = session.capture_frame().unwrap(); // drain the initial blank frame
    session.send(b"a").unwrap();

    let start = std::time::Instant::now();
    let frame = session.capture_frame().unwrap();
    let elapsed = start.elapsed();

    let any_non_background = frame.pixels().any(|p| *p != image::Rgba([0, 0, 0, 255]));
    assert!(
        any_non_background,
        "expected the echoed key text to draw something"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "expected a fast-quiescing capture to return well under MAX_SETTLE_WAIT, took {elapsed:?}"
    );
}

/// Guards a Task 12 second-round review finding: an earlier version of
/// the fix above compared each poll only against the screen's content
/// from *before* `capture_frame` was even called, rather than against
/// the immediately preceding poll taken during the same call. That
/// meant a screen that was already fully drawn and stable *before* the
/// call started — an idle wait step, a key with no visible effect,
/// capturing a static screen twice — never saw a "change" to react to,
/// so it could never be recognized as quiescent and paid the full
/// `MAX_SETTLE_WAIT` (2s) every time, even though nothing was actually
/// still drawing. This is the common case, not a rare edge case. This
/// test drives exactly that: capture once to let the fixture's echoed
/// "a" settle, then capture again immediately with nothing further
/// happening, and asserts the second call returns fast rather than
/// paying the full bound.
#[test]
fn capture_frame_is_fast_when_the_screen_is_already_stable() {
    let mut session = Session::spawn(&echo_key_binary(), 5, 40).unwrap();
    let _ = session.capture_frame().unwrap(); // initial blank frame
    session.send(b"a").unwrap();
    let _ = session.capture_frame().unwrap(); // let the echoed "a" settle

    let start = std::time::Instant::now();
    let _ = session.capture_frame().unwrap(); // nothing new happened
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(200),
        "expected an already-stable screen to be recognized as quiescent almost \
         immediately, not pay the full MAX_SETTLE_WAIT; took {elapsed:?}"
    );
}
