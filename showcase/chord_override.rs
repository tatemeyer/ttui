//! Override Sequence — enter Left, Right, Left, Right (deliberately
//! distinct from falcon's Up,Up,Down,Down chord) to unlock "Turbo
//! Grip": a GlitchBuffer::with_alpha power-up flash plus a triumphant
//! mascot reaction, then auto-returns to the menu.

use crossterm::event::{Event, KeyCode};
use crossterm::style::Color;
use std::time::Duration;
use ttui::buffer::{Cell, LayerStack};
use ttui::glitch::GlitchBuffer;
use ttui::input::{InputBinder, KeyPress};
use ttui::layout::Rect;
use ttui::theme::Theme;
use ttui::transition::Transition;
use ttui::widgets::text::Text;

const CHORD_TIMEOUT: Duration = Duration::from_millis(1500);
const FLASH_DURATION: Duration = Duration::from_millis(500);
const POST_UNLOCK_HOLD: Duration = Duration::from_millis(1500);
const PROMPT: &str = "Enter: Left, Right, Left, Right";
const UNLOCKED_TEXT: &str = "TURBO GRIP ONLINE";

#[derive(Clone, Copy, PartialEq)]
enum OverrideAction {
    Unlock,
}

pub(crate) struct OverrideSequenceState {
    input: InputBinder<OverrideAction>,
    unlocked: bool,
    pending_reaction: bool,
    flash: GlitchBuffer,
    hold: Option<Transition>,
    tick_count: u64,
}

impl OverrideSequenceState {
    pub(crate) fn new() -> Self {
        let mut input = InputBinder::new(CHORD_TIMEOUT);
        input.bind(
            vec![
                KeyPress::plain(KeyCode::Left),
                KeyPress::plain(KeyCode::Right),
                KeyPress::plain(KeyCode::Left),
                KeyPress::plain(KeyCode::Right),
            ],
            OverrideAction::Unlock,
        );
        OverrideSequenceState {
            input,
            unlocked: false,
            pending_reaction: false,
            flash: GlitchBuffer::new().with_alpha(0.5),
            hold: None,
            tick_count: 0,
        }
    }

    pub(crate) fn handle_key(&mut self, event: &Event) {
        if self.unlocked {
            return;
        }
        if self.input.feed(event) == Some(OverrideAction::Unlock) {
            self.unlocked = true;
            self.pending_reaction = true;
            self.flash.trigger(FLASH_DURATION);
            self.hold = Some(Transition::start(POST_UNLOCK_HOLD));
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration) {
        self.tick_count += 1;
        self.input.expire(elapsed);
        self.flash.tick(elapsed);
        if let Some(t) = &mut self.hold {
            t.tick(elapsed);
        }
    }

    /// One-shot: true exactly once, right after the chord unlocks.
    pub(crate) fn take_reaction(&mut self) -> bool {
        std::mem::take(&mut self.pending_reaction)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.hold.as_ref().map(|t| t.is_complete()).unwrap_or(false)
    }

    pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack) {
        let text = if self.unlocked { UNLOCKED_TEXT } else { PROMPT };
        let color = if self.unlocked {
            theme.accent
        } else {
            theme.secondary
        };
        let cx = area.x + area.width.saturating_sub(text.chars().count() as u16) / 2;
        let cy = area.y + area.height / 2;
        for (i, ch) in text.chars().enumerate() {
            let x = cx + i as u16;
            if x >= area.x + area.width {
                break;
            }
            buf.set(
                x,
                cy,
                Cell {
                    symbol: ch,
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
        }
        if self.flash.is_active() {
            let overlay = buf.push_layer();
            self.flash
                .render(area, theme.accent, self.tick_count, overlay);
        }
        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Esc back * Left Right Left Right").render(hint_row, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn the_full_chord_unlocks() {
        let mut s = OverrideSequenceState::new();
        s.handle_key(&press(KeyCode::Left));
        s.handle_key(&press(KeyCode::Right));
        s.handle_key(&press(KeyCode::Left));
        assert!(!s.unlocked);
        s.handle_key(&press(KeyCode::Right));
        assert!(s.unlocked);
    }

    #[test]
    fn an_incomplete_chord_does_not_unlock() {
        let mut s = OverrideSequenceState::new();
        s.handle_key(&press(KeyCode::Left));
        s.handle_key(&press(KeyCode::Right));
        assert!(!s.unlocked);
    }

    #[test]
    fn unlocking_sets_a_one_shot_reaction_flag() {
        let mut s = OverrideSequenceState::new();
        for code in [KeyCode::Left, KeyCode::Right, KeyCode::Left, KeyCode::Right] {
            s.handle_key(&press(code));
        }
        assert!(s.take_reaction());
        assert!(!s.take_reaction());
    }

    #[test]
    fn is_complete_only_after_the_post_unlock_hold_elapses() {
        let mut s = OverrideSequenceState::new();
        for code in [KeyCode::Left, KeyCode::Right, KeyCode::Left, KeyCode::Right] {
            s.handle_key(&press(code));
        }
        assert!(!s.is_complete());
        s.on_tick(POST_UNLOCK_HOLD);
        assert!(s.is_complete());
    }

    #[test]
    fn never_unlocked_is_never_complete() {
        let s = OverrideSequenceState::new();
        assert!(!s.is_complete());
    }
}
