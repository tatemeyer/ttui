//! App color palette and border glyph set — passed explicitly to
//! `Block`/`SmashBorder`; other widgets take plain color params
//! instead of a whole `Theme`.

use crate::buffer::CellStyle;
use crossterm::style::Color;

/// The glyphs a bordered widget draws its edges/corners with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSet {
    /// Top/bottom edge glyph.
    pub horizontal: char,
    /// Left/right edge glyph.
    pub vertical: char,
    /// Top-left corner glyph.
    pub top_left: char,
    /// Top-right corner glyph.
    pub top_right: char,
    /// Bottom-left corner glyph.
    pub bottom_left: char,
    /// Bottom-right corner glyph.
    pub bottom_right: char,
}

impl BorderSet {
    /// Real box-drawing glyphs (`┌┐└┘─│`) — the default border look.
    pub const fn single_line() -> Self {
        BorderSet {
            horizontal: '─',
            vertical: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
        }
    }

    /// Plain ASCII (`-|+`), the same `+` at every corner — for apps
    /// that want the pre-1.0 look.
    pub const fn ascii() -> Self {
        BorderSet {
            horizontal: '-',
            vertical: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        }
    }
}

impl Default for BorderSet {
    fn default() -> Self {
        Self::single_line()
    }
}

/// An app's color palette and border style, passed to `Block`/
/// `SmashBorder` renders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Base background color.
    pub background: Color,
    /// Main accent/brand color.
    pub primary: Color,
    /// Secondary accent color.
    pub secondary: Color,
    /// Tertiary accent color.
    pub tertiary: Color,
    /// Highlight/selection color.
    pub accent: Color,
    /// When set, `Block` ramps the border ring's color from `primary`
    /// toward this along a top-left-to-bottom-right diagonal across
    /// the border's bounding box, clamping to this color past the
    /// diagonal — NOT a true perimeter/arc-length gradient, so
    /// roughly the bottom-right half of a typical ring renders as a
    /// flat `primary_end` rather than a visible ramp. Both `primary`
    /// and this color must be `Color::Rgb` for the lerp to actually
    /// interpolate; `easing::lerp_color`'s existing fallback returns
    /// the target color outright for any other color type, so a
    /// non-`Rgb` `primary`/`primary_end` pair renders as a flat
    /// `primary_end` color, not a gradient.
    pub primary_end: Option<Color>,
    /// Border glyph set.
    pub border: BorderSet,
    /// Style applied to every border cell (not title cells — see
    /// `Block::render`). Reuses `Cell`'s own style type rather than a
    /// narrower bool, so future border attributes (underline, etc.)
    /// need no further `Theme` field growth.
    pub border_style: CellStyle,
    /// Whether `Block` draws an outward second border ring.
    pub border_thick: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: Color::Reset,
            primary: Color::Reset,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet::default(),
            border_style: CellStyle::default(),
            border_thick: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_uses_real_box_drawing_glyphs() {
        let b = BorderSet::single_line();
        assert_eq!(b.horizontal, '─');
        assert_eq!(b.vertical, '│');
        assert_eq!(b.top_left, '┌');
        assert_eq!(b.top_right, '┐');
        assert_eq!(b.bottom_left, '└');
        assert_eq!(b.bottom_right, '┘');
    }

    #[test]
    fn ascii_uses_a_plus_at_every_corner() {
        let b = BorderSet::ascii();
        assert_eq!(b.horizontal, '-');
        assert_eq!(b.vertical, '|');
        assert_eq!(b.top_left, '+');
        assert_eq!(b.top_right, '+');
        assert_eq!(b.bottom_left, '+');
        assert_eq!(b.bottom_right, '+');
    }

    #[test]
    fn default_matches_single_line() {
        assert_eq!(BorderSet::default(), BorderSet::single_line());
    }

    #[test]
    fn default_theme_uses_reset_colors_and_default_border() {
        let t = Theme::default();
        assert_eq!(t.background, Color::Reset);
        assert_eq!(t.primary, Color::Reset);
        assert_eq!(t.secondary, Color::Reset);
        assert_eq!(t.tertiary, Color::Reset);
        assert_eq!(t.accent, Color::Reset);
        assert_eq!(t.border, BorderSet::default());
    }

    #[test]
    fn default_theme_border_style_is_default() {
        assert_eq!(Theme::default().border_style, CellStyle::default());
    }

    #[test]
    fn default_theme_border_thick_is_false() {
        assert!(!Theme::default().border_thick);
    }

    #[test]
    fn default_theme_primary_end_is_none() {
        assert_eq!(Theme::default().primary_end, None);
    }
}
