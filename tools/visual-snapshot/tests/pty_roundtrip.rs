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
