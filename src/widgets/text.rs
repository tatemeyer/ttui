use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct Text<'a> {
    content: &'a str,
    fg: Color,
    bg: Color,
}

impl<'a> Text<'a> {
    pub fn new(content: &'a str) -> Self {
        Text {
            content,
            fg: Color::Reset,
            bg: Color::Reset,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        for (i, ch) in self.content.chars().take(area.width as usize).enumerate() {
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol: ch,
                    fg: self.fg,
                    bg: self.bg,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;

    #[test]
    fn renders_characters_left_to_right() {
        let mut buf = Buffer::new(5, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 1,
        };

        Text::new("hi").render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'h');
        assert_eq!(buf.get(1, 0).symbol, 'i');
        assert_eq!(buf.get(2, 0).symbol, ' '); // untouched, still default
    }

    #[test]
    fn truncates_content_wider_than_the_area() {
        let mut buf = Buffer::new(2, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        };

        Text::new("hello").render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'h');
        assert_eq!(buf.get(1, 0).symbol, 'e');
    }

    #[test]
    fn does_not_panic_on_zero_height_rect() {
        let mut buf = Buffer::new(5, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 0,
        };

        Text::new("hello").render(area, &mut buf);
        // Should return without panicking; buffer is untouched
    }

    #[test]
    fn does_not_panic_on_zero_width_rect() {
        let mut buf = Buffer::new(1, 5);
        let area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 5,
        };

        Text::new("hello").render(area, &mut buf);
        // Should return without panicking; buffer is untouched
    }
}
