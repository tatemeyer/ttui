//! Framework-level key-binding resolver: single keys and multi-key
//! chords resolve to an app-defined action type. Apps compose an
//! `InputBinder` into their own state (like `GlitchBuffer`/
//! `ParticleSystem`) rather than the `App` trait changing shape.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

/// A single key press to match against: code + modifiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct KeyPress {
    /// The key code (character, Tab, arrow, etc.).
    pub code: KeyCode,
    /// Required modifiers — matched via "actual contains required",
    /// not exact equality (see `InputBinder::feed`).
    pub modifiers: KeyModifiers,
}

impl KeyPress {
    /// A key press with no required modifiers — the common case.
    pub fn plain(code: KeyCode) -> Self {
        KeyPress {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl From<KeyPress> for Vec<KeyPress> {
    fn from(k: KeyPress) -> Self {
        vec![k]
    }
}

fn key_press_matches(actual: KeyPress, required: KeyPress) -> bool {
    actual.code == required.code && actual.modifiers.contains(required.modifiers)
}

fn sequence_starts_with(binding: &[KeyPress], seq: &[KeyPress]) -> bool {
    seq.len() <= binding.len()
        && seq
            .iter()
            .zip(binding.iter())
            .all(|(seq_key, binding_key)| key_press_matches(*seq_key, *binding_key))
}

fn binding_matches(binding: &[KeyPress], seq: &[KeyPress]) -> bool {
    binding.len() == seq.len() && sequence_starts_with(binding, seq)
}

/// Resolves key presses (single keys or multi-key chords) to an
/// app-defined action `A`. Compose into app state and feed it every
/// input event; call `expire` every tick to clear a stale in-progress
/// chord.
pub struct InputBinder<A: Copy> {
    bindings: Vec<(Vec<KeyPress>, A)>,
    pending: Vec<KeyPress>,
    pending_elapsed: Duration,
    chord_timeout: Duration,
}

impl<A: Copy> InputBinder<A> {
    /// `chord_timeout` bounds how long a partial chord waits for its
    /// next key before resetting.
    pub fn new(chord_timeout: Duration) -> Self {
        InputBinder {
            bindings: Vec::new(),
            pending: Vec::new(),
            pending_elapsed: Duration::ZERO,
            chord_timeout,
        }
    }

    /// Registers a binding — a single `KeyPress` (auto-converted to a
    /// length-1 sequence) or an explicit `Vec<KeyPress>` chord. If a
    /// registered binding is itself a strict prefix of a longer registered
    /// binding, the shorter one always wins and the longer one can never
    /// fire, since exact-match is checked before prefix-match at every step.
    pub fn bind(&mut self, sequence: impl Into<Vec<KeyPress>>, action: A) -> &mut Self {
        let sequence = sequence.into();
        debug_assert!(
            !sequence.is_empty(),
            "InputBinder::bind: sequence must not be empty"
        );
        self.bindings.push((sequence, action));
        self
    }

    /// Feeds one input event. Returns the resolved action once a full
    /// binding matches; `None` while a chord is still in progress or
    /// the event doesn't extend toward any binding. Ignores anything
    /// that isn't a `KeyEventKind::Press` key event.
    pub fn feed(&mut self, event: &Event) -> Option<A> {
        let Event::Key(key) = event else { return None };
        if key.kind != KeyEventKind::Press {
            return None;
        }
        let kp = KeyPress {
            code: key.code,
            modifiers: key.modifiers,
        };

        let mut candidate = self.pending.clone();
        candidate.push(kp);
        if let Some(action) = self.exact_match(&candidate) {
            self.pending.clear();
            self.pending_elapsed = Duration::ZERO;
            return Some(action);
        }
        if self.has_prefix_match(&candidate) {
            self.pending = candidate;
            self.pending_elapsed = Duration::ZERO;
            return None;
        }

        let fresh = vec![kp];
        if let Some(action) = self.exact_match(&fresh) {
            self.pending.clear();
            self.pending_elapsed = Duration::ZERO;
            return Some(action);
        }
        if self.has_prefix_match(&fresh) {
            self.pending = fresh;
        } else {
            self.pending.clear();
        }
        self.pending_elapsed = Duration::ZERO;
        None
    }

    /// Clears a pending chord once `chord_timeout` has elapsed since
    /// its last extending keypress. A no-op when nothing is pending.
    pub fn expire(&mut self, elapsed: Duration) {
        if self.pending.is_empty() {
            return;
        }
        self.pending_elapsed += elapsed;
        if self.pending_elapsed >= self.chord_timeout {
            self.pending.clear();
            self.pending_elapsed = Duration::ZERO;
        }
    }

    /// Discards any in-progress chord immediately, without waiting for
    /// `chord_timeout`. Useful when an app's mode changes in a way that
    /// should invalidate whatever the user was in the middle of typing.
    pub fn reset(&mut self) {
        self.pending.clear();
        self.pending_elapsed = Duration::ZERO;
    }

    fn exact_match(&self, seq: &[KeyPress]) -> Option<A> {
        self.bindings
            .iter()
            .find(|(binding, _)| binding_matches(binding, seq))
            .map(|(_, action)| *action)
    }

    fn has_prefix_match(&self, seq: &[KeyPress]) -> bool {
        self.bindings
            .iter()
            .any(|(binding, _)| binding.len() > seq.len() && sequence_starts_with(binding, seq))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use std::time::Duration;

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn press_with(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum TestAction {
        A,
        B,
        Chord,
    }

    #[test]
    fn single_key_binding_fires_on_first_matching_press() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        assert_eq!(binder.feed(&press(KeyCode::Char('a'))), Some(TestAction::A));
    }

    #[test]
    fn unmatched_key_returns_none() {
        let mut binder = InputBinder::<TestAction>::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        assert_eq!(binder.feed(&press(KeyCode::Char('z'))), None);
    }

    #[test]
    fn non_key_events_are_ignored() {
        let mut binder = InputBinder::<TestAction>::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        assert_eq!(binder.feed(&Event::Resize(80, 24)), None);
    }

    #[test]
    fn key_release_and_repeat_events_are_ignored() {
        use crossterm::event::KeyEventKind;
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        let release = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        assert_eq!(binder.feed(&release), None);
        let repeat = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        ));
        assert_eq!(binder.feed(&repeat), None);
    }

    #[test]
    fn two_key_chord_requires_both_keys_in_order() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "partial chord doesn't fire early"
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord)
        );
    }

    #[test]
    fn partial_chord_does_not_fire_early() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Up),
                KeyPress::plain(KeyCode::Up),
                KeyPress::plain(KeyCode::Down),
            ],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Up)), None);
        assert_eq!(
            binder.feed(&press(KeyCode::Up)),
            None,
            "still only 2 of 3 keys"
        );
    }

    #[test]
    fn abandoned_chord_prefix_falls_through_to_a_valid_single_key_binding() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        binder.bind(KeyPress::plain(KeyCode::Char('q')), TestAction::A);
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "starts the gg chord"
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('q'))),
            Some(TestAction::A),
            "q breaks the chord but still fires its own binding, not swallowed"
        );
    }

    #[test]
    fn dead_end_key_clears_pending_with_no_action() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "starts the gg chord"
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('z'))),
            None,
            "z matches nothing, no prefix — dead end, no action fires"
        );
        // Pending was cleared, so a fresh gg still works afterward:
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord)
        );
    }

    #[test]
    fn overlapping_prefix_chords_resolve_to_the_correct_one() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::A,
        );
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('h')),
            ],
            TestAction::B,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        assert_eq!(binder.feed(&press(KeyCode::Char('h'))), Some(TestAction::B));

        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), Some(TestAction::A));
    }

    #[test]
    fn modifier_matching_uses_contains_not_exact_equality() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        // Registered with NONE — should fire even if SHIFT happens to be set.
        binder.bind(KeyPress::plain(KeyCode::BackTab), TestAction::A);
        assert_eq!(
            binder.feed(&press_with(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(TestAction::A)
        );

        // Registered requiring CONTROL — must not fire without it.
        let mut ctrl_binder = InputBinder::new(Duration::from_secs(1));
        ctrl_binder.bind(
            KeyPress {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            },
            TestAction::A,
        );
        assert_eq!(ctrl_binder.feed(&press(KeyCode::Char('c'))), None);
        assert_eq!(
            ctrl_binder.feed(&press_with(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(TestAction::A)
        );
    }

    #[test]
    fn expire_does_nothing_when_pending_is_empty() {
        let mut binder = InputBinder::<TestAction>::new(Duration::from_millis(100));
        binder.expire(Duration::from_secs(10)); // no panic, no effect
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord)
        );
    }

    #[test]
    fn expire_clears_a_stale_pending_chord_after_timeout() {
        let mut binder = InputBinder::new(Duration::from_millis(100));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "starts the chord"
        );
        binder.expire(Duration::from_millis(150)); // past the 100ms timeout
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "pending was cleared by the timeout, so this is a fresh first key, not the chord's second"
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord),
            "a fresh gg from here still completes normally"
        );
    }

    #[test]
    fn expire_accumulates_across_multiple_calls_rather_than_resetting_each_time() {
        let mut binder = InputBinder::new(Duration::from_millis(100));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "starts the chord"
        );
        // Two sub-threshold calls that individually wouldn't expire it, but
        // together (30+30=60ms, still under 100ms) should not either — this
        // distinguishes real accumulation from a buggy `=` that would only
        // ever hold the most recent call's value.
        binder.expire(Duration::from_millis(30));
        binder.expire(Duration::from_millis(30));
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord),
            "60ms total is still under the 100ms timeout — chord survives"
        );
    }

    #[test]
    fn expire_accumulates_past_timeout_across_many_small_calls() {
        let mut binder = InputBinder::new(Duration::from_millis(100));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "starts the chord"
        );
        // Four 30ms calls sum to 120ms, past the 100ms timeout. A buggy `=`
        // instead of `+=` would never cross the threshold since each call
        // would overwrite (not add to) the accumulator with just 30ms.
        binder.expire(Duration::from_millis(30));
        binder.expire(Duration::from_millis(30));
        binder.expire(Duration::from_millis(30));
        binder.expire(Duration::from_millis(30));
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "pending was cleared by cumulative timeout — this is a fresh first key"
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord)
        );
    }

    #[test]
    fn expire_does_not_clear_a_chord_still_within_timeout() {
        let mut binder = InputBinder::new(Duration::from_millis(100));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        binder.expire(Duration::from_millis(50)); // under the 100ms timeout
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord),
            "chord still completes — the first key's pending state survived"
        );
    }

    #[test]
    fn reset_discards_a_pending_chord_immediately() {
        let mut binder = InputBinder::new(Duration::from_secs(10));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Char('g')),
                KeyPress::plain(KeyCode::Char('g')),
            ],
            TestAction::Chord,
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "starts the chord"
        );
        binder.reset();
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "reset cleared it — this is a fresh first key, not the chord's second"
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord)
        );
    }

    #[test]
    fn mouse_paste_and_focus_events_are_ignored() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        let mut binder = InputBinder::<TestAction>::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        let mouse = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(binder.feed(&mouse), None);
        assert_eq!(binder.feed(&Event::Paste("x".to_string())), None);
        assert_eq!(binder.feed(&Event::FocusGained), None);
        assert_eq!(binder.feed(&Event::FocusLost), None);
    }

    #[test]
    fn chord_keys_out_of_order_do_not_fire() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![KeyPress::plain(KeyCode::Up), KeyPress::plain(KeyCode::Down)],
            TestAction::Chord,
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Down)),
            None,
            "Down first doesn't start the Up,Down chord"
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Up)),
            None,
            "still nothing — Down,Up isn't a registered sequence"
        );
    }

    #[test]
    fn a_full_four_key_chord_fires_only_on_the_final_key() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Up),
                KeyPress::plain(KeyCode::Up),
                KeyPress::plain(KeyCode::Down),
                KeyPress::plain(KeyCode::Down),
            ],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Up)), None);
        assert_eq!(binder.feed(&press(KeyCode::Up)), None);
        assert_eq!(binder.feed(&press(KeyCode::Down)), None);
        assert_eq!(binder.feed(&press(KeyCode::Down)), Some(TestAction::Chord));
    }

    #[test]
    fn duplicate_registration_the_first_bound_wins() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::B);
        assert_eq!(binder.feed(&press(KeyCode::Char('a'))), Some(TestAction::A));
    }
}
