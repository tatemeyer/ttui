use crossterm::style::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSet {
    pub horizontal: char,
    pub vertical: char,
    pub corner: char,
}

impl Default for BorderSet {
    fn default() -> Self {
        BorderSet {
            horizontal: '-',
            vertical: '|',
            corner: '+',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub background: Color,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub accent: Color,
    pub border: BorderSet,
    pub border_bold: bool,
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
            border: BorderSet::default(),
            border_bold: false,
            border_thick: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_border_set_matches_todays_hardcoded_glyphs() {
        let b = BorderSet::default();
        assert_eq!(b.horizontal, '-');
        assert_eq!(b.vertical, '|');
        assert_eq!(b.corner, '+');
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
    fn default_theme_border_bold_is_false() {
        assert!(!Theme::default().border_bold);
    }

    #[test]
    fn default_theme_border_thick_is_false() {
        assert!(!Theme::default().border_thick);
    }
}
