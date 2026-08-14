//! Thick, riveted, deliberately-asymmetric double-line border for a
//! jury-rigged cockpit-panel look.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::theme::Theme;
use crossterm::style::Color;

/// A two-ring border: an outer riveted ring with one intentionally
/// mismatched corner, and a plain inner ring — colored by `focused`.
pub struct CockpitPanel {
    /// Selects `theme.primary` (bright) vs `theme.secondary` (dimmed).
    pub focused: bool,
}

impl CockpitPanel {
    /// Creates a `CockpitPanel`; `focused` selects `theme.primary`
    /// (bright) vs `theme.secondary` (dimmed) for both rings.
    pub fn new(focused: bool) -> Self {
        CockpitPanel { focused }
    }

    /// Draws the outer riveted ring and inner plain ring, returning
    /// the shrunk inner content area. Degrades to a zero-size `Rect`
    /// without panicking when `area` is too small for both rings.
    pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect {
        if area.width < 4 || area.height < 4 {
            return Rect {
                x: area.x,
                y: area.y,
                width: 0,
                height: 0,
            };
        }

        let color = if self.focused {
            theme.primary
        } else {
            theme.secondary
        };

        for x in area.x..area.x + area.width {
            let offset = x - area.x;
            let glyph = if offset % 3 == 1 { 'o' } else { '=' };
            set_cell(buf, x, area.y, glyph, color);
            set_cell(buf, x, area.y + area.height - 1, glyph, color);
        }
        for y in area.y..area.y + area.height {
            let offset = y - area.y;
            let glyph = if offset % 2 == 1 { 'o' } else { '#' };
            set_cell(buf, area.x, y, glyph, color);
            set_cell(buf, area.x + area.width - 1, y, glyph, color);
        }
        set_cell(buf, area.x, area.y, '+', color);
        set_cell(buf, area.x + area.width - 1, area.y, '+', color);
        set_cell(buf, area.x, area.y + area.height - 1, '+', color);
        set_cell(
            buf,
            area.x + area.width - 1,
            area.y + area.height - 1,
            '¤',
            color,
        );

        let inner_outer = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width - 2,
            height: area.height - 2,
        };
        for x in inner_outer.x..inner_outer.x + inner_outer.width {
            set_cell(buf, x, inner_outer.y, '-', color);
            set_cell(buf, x, inner_outer.y + inner_outer.height - 1, '-', color);
        }
        for y in inner_outer.y..inner_outer.y + inner_outer.height {
            set_cell(buf, inner_outer.x, y, '|', color);
            set_cell(buf, inner_outer.x + inner_outer.width - 1, y, '|', color);
        }
        set_cell(buf, inner_outer.x, inner_outer.y, '+', color);
        set_cell(
            buf,
            inner_outer.x + inner_outer.width - 1,
            inner_outer.y,
            '+',
            color,
        );
        set_cell(
            buf,
            inner_outer.x,
            inner_outer.y + inner_outer.height - 1,
            '+',
            color,
        );
        set_cell(
            buf,
            inner_outer.x + inner_outer.width - 1,
            inner_outer.y + inner_outer.height - 1,
            '+',
            color,
        );

        Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
        }
    }
}

fn set_cell(buf: &mut Buffer, x: u16, y: u16, symbol: char, color: Color) {
    buf.set(
        x,
        y,
        Cell {
            symbol,
            fg: color,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::CellStyle;
    use crate::theme::BorderSet;

    fn test_theme() -> Theme {
        Theme {
            background: Color::Black,
            primary: Color::Rgb {
                r: 255,
                g: 176,
                b: 0,
            },
            secondary: Color::Rgb {
                r: 76,
                g: 187,
                b: 23,
            },
            tertiary: Color::Red,
            accent: Color::Yellow,
            primary_end: None,
            border: BorderSet::default(),
            border_style: CellStyle::default(),
            border_thick: false,
        }
    }

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        }
    }

    #[test]
    fn focused_panel_uses_theme_primary_for_both_rings() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(true).render(area(), &theme, &mut buf);
        assert_eq!(buf.get(0, 0).fg, theme.primary);
        assert_eq!(buf.get(1, 1).fg, theme.primary);
    }

    #[test]
    fn unfocused_panel_uses_theme_secondary_for_both_rings() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(false).render(area(), &theme, &mut buf);
        assert_eq!(buf.get(0, 0).fg, theme.secondary);
        assert_eq!(buf.get(1, 1).fg, theme.secondary);
    }

    #[test]
    fn three_corners_are_plus_and_bottom_right_is_the_one_asymmetric_glyph() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(true).render(area(), &theme, &mut buf);
        assert_eq!(buf.get(0, 0).symbol, '+');
        assert_eq!(buf.get(9, 0).symbol, '+');
        assert_eq!(buf.get(0, 7).symbol, '+');
        assert_eq!(buf.get(9, 7).symbol, '¤');
    }

    #[test]
    fn rivets_appear_at_the_expected_deterministic_offsets_on_the_outer_ring() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(true).render(area(), &theme, &mut buf);
        // top edge (width 10): offset % 3 == 1 -> offsets 1, 4, 7 are rivets.
        assert_eq!(buf.get(1, 0).symbol, 'o');
        assert_eq!(buf.get(4, 0).symbol, 'o');
        assert_eq!(buf.get(7, 0).symbol, 'o');
        assert_eq!(buf.get(2, 0).symbol, '=');
        assert_eq!(buf.get(3, 0).symbol, '=');
        // left edge: offset % 2 == 1 -> offset 1 is a rivet, offset 2 isn't.
        assert_eq!(buf.get(0, 1).symbol, 'o');
        assert_eq!(buf.get(0, 2).symbol, '#');
    }

    #[test]
    fn inner_ring_has_no_rivets_and_no_asymmetry() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        CockpitPanel::new(true).render(area(), &theme, &mut buf);
        // inner ring: area shrunk by 1 on each side -> x in 1..9, y in 1..7.
        assert_eq!(buf.get(1, 1).symbol, '+');
        assert_eq!(buf.get(8, 1).symbol, '+');
        assert_eq!(buf.get(1, 6).symbol, '+');
        assert_eq!(buf.get(8, 6).symbol, '+'); // not asymmetric, unlike the outer ring
        assert_eq!(buf.get(2, 1).symbol, '-');
        assert_eq!(buf.get(1, 2).symbol, '|');
    }

    #[test]
    fn returns_area_shrunk_by_two_on_each_side() {
        let theme = test_theme();
        let mut buf = Buffer::new(10, 8);
        let inner = CockpitPanel::new(true).render(area(), &theme, &mut buf);
        assert_eq!(
            inner,
            Rect {
                x: 2,
                y: 2,
                width: 6,
                height: 4
            }
        );
    }

    #[test]
    fn too_small_area_degrades_gracefully_without_panic() {
        let theme = test_theme();
        let mut buf = Buffer::new(3, 3);
        let small = Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        };
        let inner = CockpitPanel::new(true).render(small, &theme, &mut buf);
        assert_eq!(
            inner,
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0
            }
        );
        assert_eq!(*buf.get(1, 1), Cell::default());
    }
}
