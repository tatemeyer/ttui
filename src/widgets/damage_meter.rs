//! Percent display that continuously ramps white → yellow → red as it
//! climbs toward (and past) 100%.

use crate::buffer::{Buffer, Cell};
use crate::easing::lerp_color;
use crate::layout::Rect;
use crossterm::style::Color;

const WHITE: Color = Color::Rgb {
    r: 255,
    g: 255,
    b: 255,
};
const YELLOW: Color = Color::Rgb {
    r: 255,
    g: 255,
    b: 0,
};
const RED: Color = Color::Rgb { r: 255, g: 0, b: 0 };

/// A "N%" text readout whose color continuously ramps white → yellow
/// → red as `percent` climbs, holding solid red from 100% up.
pub struct DamageMeter {
    percent: u16,
}

impl DamageMeter {
    /// Creates a meter showing `percent` (uncapped — can exceed 100).
    pub fn new(percent: u16) -> Self {
        DamageMeter { percent }
    }

    /// Renders the percent text left-aligned in `area`.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let color = damage_color(self.percent);
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

fn damage_color(percent: u16) -> Color {
    if percent >= 100 {
        RED
    } else if percent >= 50 {
        let t = (percent - 50) as f32 / 50.0;
        lerp_color(YELLOW, RED, t)
    } else {
        let t = percent as f32 / 50.0;
        lerp_color(WHITE, YELLOW, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::lerp_color;

    const WHITE: Color = Color::Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
    const YELLOW: Color = Color::Rgb {
        r: 255,
        g: 255,
        b: 0,
    };
    const RED: Color = Color::Rgb { r: 255, g: 0, b: 0 };

    fn area10x1() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        }
    }

    #[test]
    fn zero_percent_renders_white() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(0).render(area10x1(), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, '0');
        assert_eq!(buf.get(1, 0).symbol, '%');
        assert_eq!(buf.get(0, 0).fg, WHITE);
    }

    #[test]
    fn twenty_five_percent_is_partway_between_white_and_yellow() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(25).render(area10x1(), &mut buf);
        let fg = buf.get(0, 0).fg;
        assert_eq!(fg, lerp_color(WHITE, YELLOW, 0.5));
        assert_ne!(fg, WHITE);
        assert_ne!(fg, YELLOW);
    }

    #[test]
    fn fifty_percent_renders_exactly_yellow() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(50).render(area10x1(), &mut buf);
        assert_eq!(buf.get(0, 0).fg, YELLOW);
    }

    #[test]
    fn seventy_five_percent_is_partway_between_yellow_and_red() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(75).render(area10x1(), &mut buf);
        let fg = buf.get(0, 0).fg;
        assert_eq!(fg, lerp_color(YELLOW, RED, 0.5));
        assert_ne!(fg, YELLOW);
        assert_ne!(fg, RED);
    }

    #[test]
    fn over_100_percent_renders_red_with_full_text() {
        let mut buf = Buffer::new(10, 1);
        DamageMeter::new(137).render(area10x1(), &mut buf);
        assert_eq!(buf.get(0, 0).symbol, '1');
        assert_eq!(buf.get(1, 0).symbol, '3');
        assert_eq!(buf.get(2, 0).symbol, '7');
        assert_eq!(buf.get(3, 0).symbol, '%');
        assert_eq!(buf.get(0, 0).fg, RED);
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
