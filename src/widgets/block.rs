use crate::buffer::{Buffer, Cell, CellStyle};
use crate::layout::Rect;
use crate::theme::{BorderSet, Theme};
use crossterm::style::Color;

pub struct Block<'a> {
    title: Option<&'a str>,
    theme: Option<&'a Theme>,
}

impl<'a> Block<'a> {
    pub fn new() -> Self {
        Block {
            title: None,
            theme: None,
        }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) -> Rect {
        if area.width < 2 || area.height < 2 {
            return area;
        }
        let (border, fg, bg, border_bold, border_thick) = match self.theme {
            Some(t) => (
                t.border,
                t.primary,
                t.background,
                t.border_bold,
                t.border_thick,
            ),
            None => (
                BorderSet::default(),
                Color::Reset,
                Color::Reset,
                false,
                false,
            ),
        };
        let plain = || Cell {
            symbol: ' ',
            fg,
            bg,
            style: CellStyle { bold: border_bold },
        };

        let draw_ring = |ring_area: Rect, buf: &mut Buffer| {
            for x in ring_area.x..ring_area.x + ring_area.width {
                buf.set(
                    x,
                    ring_area.y,
                    Cell {
                        symbol: border.horizontal,
                        ..plain()
                    },
                );
                buf.set(
                    x,
                    ring_area.y + ring_area.height - 1,
                    Cell {
                        symbol: border.horizontal,
                        ..plain()
                    },
                );
            }
            for y in ring_area.y..ring_area.y + ring_area.height {
                buf.set(
                    ring_area.x,
                    y,
                    Cell {
                        symbol: border.vertical,
                        ..plain()
                    },
                );
                buf.set(
                    ring_area.x + ring_area.width - 1,
                    y,
                    Cell {
                        symbol: border.vertical,
                        ..plain()
                    },
                );
            }
            buf.set(
                ring_area.x,
                ring_area.y,
                Cell {
                    symbol: border.corner,
                    ..plain()
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                ring_area.y,
                Cell {
                    symbol: border.corner,
                    ..plain()
                },
            );
            buf.set(
                ring_area.x,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.corner,
                    ..plain()
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.corner,
                    ..plain()
                },
            );
        };

        draw_ring(area, buf);

        if border_thick {
            let outer_x = area.x.saturating_sub(1);
            let outer_y = area.y.saturating_sub(1);
            let outer_w = (area.width + 2).min(buf.width.saturating_sub(outer_x));
            let outer_h = (area.height + 2).min(buf.height.saturating_sub(outer_y));
            if outer_w >= 2 && outer_h >= 2 {
                draw_ring(
                    Rect {
                        x: outer_x,
                        y: outer_y,
                        width: outer_w,
                        height: outer_h,
                    },
                    buf,
                );
            }
        }

        if let Some(title) = self.title {
            for (i, ch) in title
                .chars()
                .take(area.width.saturating_sub(2) as usize)
                .enumerate()
            {
                buf.set(
                    area.x + 1 + i as u16,
                    area.y,
                    Cell {
                        symbol: ch,
                        style: CellStyle::default(),
                        ..plain()
                    },
                );
            }
        }

        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    }
}

impl<'a> Default for Block<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crate::theme::{BorderSet, Theme};

    #[test]
    fn draws_border_and_returns_inner_area() {
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        let inner = Block::new().render(area, &mut buf);

        assert_eq!(
            inner,
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 1
            }
        );
        assert_eq!(buf.get(0, 0).symbol, '+');
        assert_eq!(buf.get(1, 0).symbol, '-');
        assert_eq!(buf.get(0, 1).symbol, '|');
    }

    #[test]
    fn title_is_drawn_on_the_top_border() {
        let mut buf = Buffer::new(6, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 6,
            height: 3,
        };

        Block::new().title("Hi").render(area, &mut buf);

        assert_eq!(buf.get(1, 0).symbol, 'H');
        assert_eq!(buf.get(2, 0).symbol, 'i');
    }

    #[test]
    fn without_theme_border_colors_are_reset() {
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '+');
        assert_eq!(buf.get(0, 0).fg, Color::Reset);
        assert_eq!(buf.get(0, 0).bg, Color::Reset);
        assert!(!buf.get(0, 0).style.bold);
    }

    #[test]
    fn with_theme_border_uses_theme_glyphs_and_colors() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '*',
            },
            border_bold: false,
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '*'); // corner
        assert_eq!(buf.get(1, 0).symbol, '='); // horizontal
        assert_eq!(buf.get(0, 1).symbol, '#'); // vertical
        assert_eq!(buf.get(0, 0).fg, Color::Green);
        assert_eq!(buf.get(0, 0).bg, Color::Black);
    }

    #[test]
    fn border_cells_are_bold_when_theme_border_bold_is_true() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '*',
            },
            border_bold: true,
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert!(buf.get(0, 0).style.bold); // corner
        assert!(buf.get(1, 0).style.bold); // horizontal edge
        assert!(buf.get(0, 1).style.bold); // vertical edge
    }

    #[test]
    fn thick_border_draws_a_second_ring_one_cell_outward() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '*',
            },
            border_bold: false,
            border_thick: true,
        };
        let mut buf = Buffer::new(6, 5);
        let area = Rect {
            x: 1,
            y: 1,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '*'); // outer corner
        assert_eq!(buf.get(1, 0).symbol, '='); // outer top edge
        assert_eq!(buf.get(0, 1).symbol, '#'); // outer left edge
        assert_eq!(buf.get(0, 0).fg, Color::Green);
        assert_eq!(buf.get(0, 0).bg, Color::Black);
    }

    #[test]
    fn thin_border_leaves_the_outward_ring_untouched() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '*',
            },
            border_bold: false,
            border_thick: false,
        };
        let mut buf = Buffer::new(6, 5);
        let area = Rect {
            x: 1,
            y: 1,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn theme_less_border_leaves_the_outward_ring_untouched() {
        let mut buf = Buffer::new(6, 5);
        let area = Rect {
            x: 1,
            y: 1,
            width: 4,
            height: 3,
        };

        Block::new().render(area, &mut buf);

        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn title_cells_are_not_bold_even_when_theme_border_bold_is_true() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                corner: '*',
            },
            border_bold: true,
            border_thick: false,
        };
        let mut buf = Buffer::new(6, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 6,
            height: 3,
        };

        Block::new()
            .title("Hi")
            .theme(&theme)
            .render(area, &mut buf);

        assert!(!buf.get(1, 0).style.bold); // 'H'
        assert!(!buf.get(2, 0).style.bold); // 'i'
    }
}
