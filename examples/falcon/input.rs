use super::*;

const CHORD_TIMEOUT: Duration = Duration::from_millis(1500);
pub(super) const FULL_POWER_GLITCH_DURATION_MS: u64 = 500;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum FalconAction {
    FocusNext,
    FocusPrev,
    Whack,
    Quit,
    FullPower,
}

pub(super) fn falcon_input() -> InputBinder<FalconAction> {
    let mut binder = InputBinder::new(CHORD_TIMEOUT);
    binder.bind(KeyPress::plain(KeyCode::Tab), FalconAction::FocusNext);
    binder.bind(KeyPress::plain(KeyCode::BackTab), FalconAction::FocusPrev);
    binder.bind(KeyPress::plain(KeyCode::Char(' ')), FalconAction::Whack);
    binder.bind(KeyPress::plain(KeyCode::Char('q')), FalconAction::Quit);
    binder.bind(
        vec![
            KeyPress::plain(KeyCode::Up),
            KeyPress::plain(KeyCode::Up),
            KeyPress::plain(KeyCode::Down),
            KeyPress::plain(KeyCode::Down),
        ],
        FalconAction::FullPower,
    );
    binder
}
