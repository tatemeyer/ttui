//! Three-ring beveled border, drawn inward (unlike `Block`'s outward
//! second ring) for a chunky, plastic-toy look.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::theme::Theme;
use crossterm::style::Color;

/// A three-concentric-ring beveled border, each ring in a different
/// `Theme` color.
pub struct SmashBorder;

impl SmashBorder {
    /// Creates a `SmashBorder`.
    pub fn new() -> Self {
        SmashBorder
    }

    /// Draws all three rings inward from `area` and returns the
    /// shrunk inner content area.
    pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect {
        let rings: [(char, char, [char; 4], Color); 3] = [
            ('#', '#', ['#', '#', '#', '#'], theme.accent),
            (
                theme.border.horizontal,
                theme.border.vertical,
                [
                    theme.border.top_left,
                    theme.border.top_right,
                    theme.border.bottom_left,
                    theme.border.bottom_right,
                ],
                theme.primary,
            ),
            ('-', ':', ['.', '.', '.', '.'], theme.tertiary),
        ];

        let mut inner = area;
        for (h, v, corners, color) in rings {
            if inner.width < 2 || inner.height < 2 {
                break;
            }
            for x in inner.x..inner.x + inner.width {
                buf.set(
                    x,
                    inner.y,
                    Cell {
                        symbol: h,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                buf.set(
                    x,
                    inner.y + inner.height - 1,
                    Cell {
                        symbol: h,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
            for y in inner.y..inner.y + inner.height {
                buf.set(
                    inner.x,
                    y,
                    Cell {
                        symbol: v,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                buf.set(
                    inner.x + inner.width - 1,
                    y,
                    Cell {
                        symbol: v,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
            buf.set(
                inner.x,
                inner.y,
                Cell {
                    symbol: corners[0], // top-left
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x + inner.width - 1,
                inner.y,
                Cell {
                    symbol: corners[1], // top-right
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x,
                inner.y + inner.height - 1,
                Cell {
                    symbol: corners[2], // bottom-left
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x + inner.width - 1,
                inner.y + inner.height - 1,
                Cell {
                    symbol: corners[3], // bottom-right
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );

            inner = Rect {
                x: inner.x + 1,
                y: inner.y + 1,
                width: inner.width.saturating_sub(2),
                height: inner.height.saturating_sub(2),
            };
        }

        inner
    }
}

impl Default for SmashBorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::CellStyle;
    use crate::theme::BorderSet;

    fn test_theme() -> Theme {
        Theme {
            background: Color::Black,
            primary: Color::Red,
            secondary: Color::Reset,
            tertiary: Color::White,
            accent: Color::Yellow,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '|',
                top_left: '+',
                top_right: '+',
                bottom_left: '+',
                bottom_right: '+',
            },
            border_style: CellStyle::default(),
            border_thick: false,
        }
    }

    #[test]
    fn draws_three_concentric_rings_and_returns_shrunk_inner_area() {
        let theme = test_theme();
        let mut buf = Buffer::new(12, 10);
        let area = Rect {
            x: 0,
            y: 0,
            width: 12,
            height: 10,
        };

        let inner = SmashBorder::new().render(area, &theme, &mut buf);

        assert_eq!(
            inner,
            Rect {
                x: 3,
                y: 3,
                width: 6,
                height: 4
            }
        );

        // outer ring: '#' in theme.accent
        assert_eq!(buf.get(0, 0).symbol, '#');
        assert_eq!(buf.get(0, 0).fg, Color::Yellow);
        assert_eq!(buf.get(1, 0).symbol, '#');
        assert_eq!(buf.get(1, 0).fg, Color::Yellow);

        // middle ring: theme.border glyphs in theme.primary
        assert_eq!(buf.get(1, 1).symbol, '+');
        assert_eq!(buf.get(1, 1).fg, Color::Red);
        assert_eq!(buf.get(2, 1).symbol, '=');
        assert_eq!(buf.get(2, 1).fg, Color::Red);

        // inner ring: '-'/':'/'.' in theme.tertiary
        assert_eq!(buf.get(2, 2).symbol, '.');
        assert_eq!(buf.get(2, 2).fg, Color::White);
        assert_eq!(buf.get(3, 2).symbol, '-');
        assert_eq!(buf.get(3, 2).fg, Color::White);
    }

    #[test]
    fn too_small_area_degrades_gracefully_without_panic() {
        let theme = test_theme();
        let mut buf = Buffer::new(3, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        };

        let inner = SmashBorder::new().render(area, &theme, &mut buf);

        assert_eq!(
            inner,
            Rect {
                x: 1,
                y: 1,
                width: 1,
                height: 1
            }
        );
        assert_eq!(*buf.get(1, 1), Cell::default());
    }

    #[test]
    fn middle_ring_renders_all_four_corners_from_their_own_field() {
        let mut theme = test_theme();
        theme.border = BorderSet {
            horizontal: '=',
            vertical: '|',
            top_left: '1',
            top_right: '2',
            bottom_left: '3',
            bottom_right: '4',
        };
        let mut buf = Buffer::new(12, 10);
        let area = Rect {
            x: 0,
            y: 0,
            width: 12,
            height: 10,
        };

        SmashBorder::new().render(area, &theme, &mut buf);

        // Middle ring sits 1 cell inward from the outer ring's own
        // bounds (0..12, 0..10) -> middle ring spans (1..11, 1..9).
        assert_eq!(buf.get(1, 1).symbol, '1'); // top-left
        assert_eq!(buf.get(10, 1).symbol, '2'); // top-right
        assert_eq!(buf.get(1, 8).symbol, '3'); // bottom-left
        assert_eq!(buf.get(10, 8).symbol, '4'); // bottom-right
    }
}
