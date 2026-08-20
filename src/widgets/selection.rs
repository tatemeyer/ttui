//! Shared selection-highlight colour resolution for the selectable
//! widgets (`List`, `Dial`, `Table`), which each previously hardcoded
//! the same black-on-white pair and took no colours at all.

use crate::theme::Theme;
use crossterm::style::Color;

/// Resolves the `(fg, bg)` pair for one row of a selectable widget.
/// Without a `Theme`, returns the fixed pre-2.0 black-on-white
/// highlight, so an untouched call site renders exactly as 1.x did.
pub(crate) fn selection_colors(theme: Option<&Theme>, selected: bool) -> (Color, Color) {
    match (theme, selected) {
        (Some(t), true) => (t.accent, t.background),
        (Some(t), false) => (t.primary, Color::Reset),
        (None, true) => (Color::Black, Color::White),
        (None, false) => (Color::Reset, Color::Reset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::test_support::test_theme;

    #[test]
    fn without_a_theme_the_pre_2_0_colours_are_used() {
        assert_eq!(selection_colors(None, true), (Color::Black, Color::White));
        assert_eq!(selection_colors(None, false), (Color::Reset, Color::Reset));
    }

    #[test]
    fn with_a_theme_selection_is_accent_on_background() {
        let t = test_theme();
        assert_eq!(selection_colors(Some(&t), true), (t.accent, t.background));
    }

    #[test]
    fn with_a_theme_an_unselected_row_is_primary_on_reset() {
        let t = test_theme();
        assert_eq!(selection_colors(Some(&t), false), (t.primary, Color::Reset));
    }
}
