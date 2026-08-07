use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct DamageMeter {
    percent: u16,
}

impl DamageMeter {
    pub fn new(percent: u16) -> Self {
        DamageMeter { percent }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let color = if self.percent >= 100 {
            Color::Red
        } else if self.percent >= 50 {
            Color::Yellow
        } else {
            Color::White
        };
        let text = format!("{}%", self.percent);
        for (i, ch) in text.chars().take(area.width as usize).enumerate() {
            buf.set(
                area.x + i as u16,
                area.y,
                Cell {
                    symbol: ch,
                    fg: color,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_percent_renders_white() {
        let mut buf = Buffer::new(10, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        };

        DamageMeter::new(0).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '0');
        assert_eq!(buf.get(1, 0).symbol, '%');
        assert_eq!(buf.get(0, 0).fg, Color::White);
    }

    #[test]
    fn fifty_percent_renders_yellow() {
        let mut buf = Buffer::new(10, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        };

        DamageMeter::new(50).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).fg, Color::Yellow);
    }

    #[test]
    fn over_100_percent_renders_red_with_full_text() {
        let mut buf = Buffer::new(10, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        };

        DamageMeter::new(137).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '1');
        assert_eq!(buf.get(1, 0).symbol, '3');
        assert_eq!(buf.get(2, 0).symbol, '7');
        assert_eq!(buf.get(3, 0).symbol, '%');
        assert_eq!(buf.get(0, 0).fg, Color::Red);
    }

    #[test]
    fn text_wider_than_area_clips_without_panic() {
        let mut buf = Buffer::new(2, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        };

        DamageMeter::new(137).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '1');
        assert_eq!(buf.get(1, 0).symbol, '3');
    }
}
