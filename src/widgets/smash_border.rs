use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::theme::Theme;
use crossterm::style::Color;

pub struct SmashBorder;

impl SmashBorder {
    pub fn new() -> Self {
        SmashBorder
    }

    pub fn render(&self, area: Rect, theme: &Theme, buf: &mut Buffer) -> Rect {
        let rings: [(char, char, char, Color); 3] = [
            ('#', '#', '#', theme.accent),
            (
                theme.border.horizontal,
                theme.border.vertical,
                theme.border.corner,
                theme.primary,
            ),
            ('-', ':', '.', theme.tertiary),
        ];

        let mut inner = area;
        for (h, v, c, color) in rings {
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
                        ..Default::default()
                    },
                );
            }
            buf.set(
                inner.x,
                inner.y,
                Cell {
                    symbol: c,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x + inner.width - 1,
                inner.y,
                Cell {
                    symbol: c,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x,
                inner.y + inner.height - 1,
                Cell {
                    symbol: c,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            buf.set(
                inner.x + inner.width - 1,
                inner.y + inner.height - 1,
                Cell {
                    symbol: c,
                    fg: color,
                    bg: Color::Reset,
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
    use crate::theme::BorderSet;

    fn test_theme() -> Theme {
        Theme {
            background: Color::Black,
            primary: Color::Red,
            secondary: Color::Reset,
            tertiary: Color::White,
            accent: Color::Yellow,
            border: BorderSet {
                horizontal: '=',
                vertical: '|',
                corner: '+',
            },
            border_bold: false,
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
}
