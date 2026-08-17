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
    assert!(status.unwrap().success(), "fixture exited non-zero");
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
    assert!(status.unwrap().success(), "fixture exited non-zero");
    let contents = std::fs::read_to_string(&out_path).unwrap();
    assert!(contents.contains("panicked=true"), "{contents}");
    assert!(contents.contains("raw_before_panic=true"), "{contents}");
    assert!(contents.contains("raw_after_panic=false"), "{contents}");
}

/// ttui#30: an app driven only by `on_tick` must be able to exit. Before
/// the fix, `run`'s loop consulted `should_quit()` only in its input arm,
/// so this fixture — which never receives a keypress — polled forever.
#[test]
fn an_app_that_quits_from_on_tick_exits_without_any_input() {
    let dir = tempdir().unwrap();
    let out_path = dir.path().join("result.txt");
    let mut session = Session::spawn_with_args(
        &fixture_binary("tick_exit_fixture"),
        5,
        40,
        &[out_path.to_str().unwrap()],
    )
    .unwrap();
    let status = session.wait_for_exit(Duration::from_secs(5));
    assert!(
        status.is_some(),
        "fixture never exited: on_tick cannot trigger app exit"
    );
    assert!(status.unwrap().success(), "fixture exited non-zero");
    let contents = std::fs::read_to_string(&out_path).unwrap();
    assert!(contents.contains("quit=true"), "{contents}");
    assert!(contents.contains("ticks=3"), "{contents}");
}
