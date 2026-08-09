//! Pulsing circular decoration glyph — a single-cell glyph
//! (`radius: 0`) or a real filled circle (`radius >= 1`) whose
//! brightness the owning app drives per frame via `intensity`.

use crate::buffer::{Buffer, Cell};
use crate::canvas::{Canvas, CanvasMode};
use crate::layout::Rect;
use crossterm::style::Color;

/// A glyph, or a filled circle at `radius >= 1`, whose
/// brightness/fill reflects `intensity`.
pub struct Roundel {
    intensity: f32,
    color: Color,
    radius: u16,
}

impl Roundel {
    /// Creates a roundel at `intensity` (clamped to `0.0..=1.0`) in
    /// `color`. `radius: 0` renders the original single-glyph `'O'`
    /// at `area`'s center (unchanged from before this widget grew a
    /// circle mode); `radius >= 1` renders a filled circle roughly
    /// `radius * 2 + 1` cells across, via `Canvas`.
    pub fn new(intensity: f32, color: Color, radius: u16) -> Self {
        Roundel {
            intensity: intensity.clamp(0.0, 1.0),
            color,
            radius,
        }
    }

    /// Renders at `area`'s center: a single glyph at `radius: 0`, or
    /// a filled circle at `radius >= 1`.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let scaled = scale_color(self.color, self.intensity);
        if self.radius == 0 {
            let cx = area.x + area.width / 2;
            let cy = area.y + area.height / 2;
            buf.set(
                cx,
                cy,
                Cell {
                    symbol: 'O',
                    fg: scaled,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
            return;
        }
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let grid_w = area.width * 2;
        let grid_h = area.height * 4;
        let cx = grid_w as f32 / 2.0;
        let cy = grid_h as f32 / 2.0;
        // A typical monospace cell is roughly twice as tall as wide
        // in real pixels, and braille's 2-wide x 4-tall dot grid
        // divides each cell in exactly that ratio — so each dot is
        // roughly square in real screen space, and a plain Euclidean
        // distance in dot-coordinate space already reads as round
        // without any aspect-ratio correction.
        let subpixel_radius = self.radius as f32 * 2.0 + 1.0;
        for y in 0..grid_h {
            for x in 0..grid_w {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if (dx * dx + dy * dy).sqrt() <= subpixel_radius {
                    canvas.set_pixel(x, y, scaled);
                }
            }
        }
        canvas.blit(buf, area.x, area.y);
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
    fn radius_zero_zero_intensity_renders_near_black() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            0,
        )
        .render(area3x3(), &mut buf);

        assert_eq!(buf.get(1, 1).fg, Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn radius_zero_full_intensity_renders_the_input_color_unchanged() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            1.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            0,
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
    fn radius_zero_half_intensity_halves_each_channel() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.5,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            0,
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
    fn radius_zero_renders_at_area_center_and_does_not_panic_on_a_1x1_area() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };

        Roundel::new(1.0, Color::White, 0).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, 'O');
    }

    #[test]
    fn radius_one_renders_a_circle_spanning_multiple_cells() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            1.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            1,
        )
        .render(area3x3(), &mut buf);

        let mut non_default = 0;
        for y in 0..3 {
            for x in 0..3 {
                if *buf.get(x, y) != Cell::default() {
                    non_default += 1;
                }
            }
        }
        assert!(
            non_default > 1,
            "expected a circle spanning multiple cells, got {non_default}"
        );
    }

    #[test]
    fn larger_radius_marks_strictly_more_cells_than_a_smaller_one() {
        let mut small = Buffer::new(5, 5);
        let mut large = Buffer::new(5, 5);
        let area = Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 5,
        };
        let color = Color::Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        Roundel::new(1.0, color, 1).render(area, &mut small);
        Roundel::new(1.0, color, 2).render(area, &mut large);

        let count = |buf: &Buffer| -> usize {
            let mut n = 0;
            for y in 0..5 {
                for x in 0..5 {
                    if *buf.get(x, y) != Cell::default() {
                        n += 1;
                    }
                }
            }
            n
        };
        assert!(
            count(&large) > count(&small),
            "radius 2 should mark more cells than radius 1"
        );
    }

    #[test]
    fn radius_one_zero_intensity_still_renders_near_black() {
        let mut buf = Buffer::new(3, 3);
        Roundel::new(
            0.0,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            },
            1,
        )
        .render(area3x3(), &mut buf);

        assert_eq!(buf.get(1, 1).fg, Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn radius_one_on_a_tiny_area_does_not_panic() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        Roundel::new(1.0, Color::White, 1).render(area, &mut buf);
    }
}
