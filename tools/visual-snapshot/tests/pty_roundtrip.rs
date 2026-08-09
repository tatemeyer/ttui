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
