use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

pub struct Roundel {
    intensity: f32,
    color: Color,
}

impl Roundel {
    pub fn new(intensity: f32, color: Color) -> Self {
        Roundel {
            intensity: intensity.clamp(0.0, 1.0),
            color,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let cx = area.x + area.width / 2;
        let cy = area.y + area.height / 2;
        buf.set(
            cx,
            cy,
            Cell {
                symbol: 'O',
                fg: scale_color(self.color, self.intensity),
                bg: Color::Reset,
                ..Default::default()
            },
        );
    }
}

fn scale_color(c: Color, intensity: f32) -> Color {
    match c {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f32 * intensity) as u8,
            g: (g as f32 * intensity) as u8,
            b: (b as f32 * intensity) as u8,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area3x3() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        }
    }

    #[test]
    fn zero_intensity_renders_near_black() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
        )
        .render(area3x3(), &mut buf);

        assert_eq!(buf.get(1, 1).fg, Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn full_intensity_renders_the_input_color_unchanged() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            1.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
        )
        .render(area3x3(), &mut buf);

        assert_eq!(
            buf.get(1, 1).fg,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50
            }
        );
    }

    #[test]
    fn half_intensity_halves_each_channel() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.5,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
        )
        .render(area3x3(), &mut buf);

        assert_eq!(
            buf.get(1, 1).fg,
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
    }

    #[test]
    fn renders_at_area_center_and_does_not_panic_on_a_1x1_area() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };

        Roundel::new(1.0, Color::White).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'O');
    }
}
