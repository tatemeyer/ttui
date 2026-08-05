use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct Block<'a> {
    title: Option<&'a str>,
}

impl<'a> Block<'a> {
    pub fn new() -> Self {
        Block { title: None }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) -> Rect {
        if area.width < 2 || area.height < 2 {
            return area;
        }
        let plain = || Cell {
            symbol: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
        };
        for x in area.x..area.x + area.width {
            buf.set(
                x,
                area.y,
                Cell {
                    symbol: '-',
                    ..plain()
                },
            );
            buf.set(
                x,
                area.y + area.height - 1,
                Cell {
                    symbol: '-',
                    ..plain()
                },
            );
        }
        for y in area.y..area.y + area.height {
            buf.set(
                area.x,
                y,
                Cell {
                    symbol: '|',
                    ..plain()
                },
            );
            buf.set(
                area.x + area.width - 1,
                y,
                Cell {
                    symbol: '|',
                    ..plain()
                },
            );
        }
        buf.set(
            area.x,
            area.y,
            Cell {
                symbol: '+',
                ..plain()
            },
        );
        buf.set(
            area.x + area.width - 1,
            area.y,
            Cell {
                symbol: '+',
                ..plain()
            },
        );
        buf.set(
            area.x,
            area.y + area.height - 1,
            Cell {
                symbol: '+',
                ..plain()
            },
        );
        buf.set(
            area.x + area.width - 1,
            area.y + area.height - 1,
            Cell {
                symbol: '+',
                ..plain()
            },
        );

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
}
