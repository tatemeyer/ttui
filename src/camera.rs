use crate::buffer::{Buffer, Cell};
use crossterm::style::Color;

pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Camera {
    pub fn new(x: f32, y: f32, zoom: f32) -> Self {
        Camera { x, y, zoom }
    }
}

pub fn viewport(source: &Buffer, camera: &Camera, width: u16, height: u16) -> Buffer {
    let mut out = Buffer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let src_x = (camera.x + x as f32 / camera.zoom).floor();
            let src_y = (camera.y + y as f32 / camera.zoom).floor();
            if src_x >= 0.0
                && src_y >= 0.0
                && (src_x as u16) < source.width
                && (src_y as u16) < source.height
            {
                out.set(x, y, source.get(src_x as u16, src_y as u16).clone());
            }
        }
    }
    out
}

pub fn dim(buf: &Buffer, factor: f32) -> Buffer {
    let factor = factor.clamp(0.0, 1.0);
    let mut out = Buffer::new(buf.width, buf.height);
    for y in 0..buf.height {
        for x in 0..buf.width {
            let cell = buf.get(x, y);
            out.set(
                x,
                y,
                Cell {
                    fg: scale_color(cell.fg, factor),
                    bg: scale_color(cell.bg, factor),
                    ..cell.clone()
                },
            );
        }
    }
    out
}

fn scale_color(c: Color, factor: f32) -> Color {
    match c {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f32 * (1.0 - factor)) as u8,
            g: (g as f32 * (1.0 - factor)) as u8,
            b: (b as f32 * (1.0 - factor)) as u8,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labeled(symbol: char) -> Cell {
        Cell {
            symbol,
            ..Default::default()
        }
    }

    #[test]
    fn viewport_at_zoom_one_crops_a_window_at_the_camera_position() {
        let mut source = Buffer::new(10, 10);
        source.set(3, 4, labeled('A'));
        source.set(4, 4, labeled('B'));
        let camera = Camera::new(3.0, 4.0, 1.0);

        let out = viewport(&source, &camera, 5, 3);

        assert_eq!(out.get(0, 0).symbol, 'A');
        assert_eq!(out.get(1, 0).symbol, 'B');
    }

    #[test]
    fn viewport_at_zoom_two_duplicates_each_source_cell_across_two_output_cells() {
        let mut source = Buffer::new(4, 1);
        source.set(0, 0, labeled('A'));
        source.set(1, 0, labeled('B'));
        let camera = Camera::new(0.0, 0.0, 2.0);

        let out = viewport(&source, &camera, 4, 1);

        assert_eq!(out.get(0, 0).symbol, 'A');
        assert_eq!(out.get(1, 0).symbol, 'A');
        assert_eq!(out.get(2, 0).symbol, 'B');
        assert_eq!(out.get(3, 0).symbol, 'B');
    }

    #[test]
    fn viewport_with_an_out_of_bounds_camera_does_not_panic() {
        let source = Buffer::new(4, 4);
        let camera = Camera::new(-5.0, -5.0, 1.0);

        let out = viewport(&source, &camera, 3, 3);

        assert_eq!(*out.get(0, 0), Cell::default());
    }

    #[test]
    fn dim_at_factor_zero_leaves_rgb_cells_unchanged() {
        let mut buf = Buffer::new(1, 1);
        buf.set(
            0,
            0,
            Cell {
                symbol: 'X',
                fg: Color::Rgb {
                    r: 100,
                    g: 150,
                    b: 200,
                },
                bg: Color::Rgb {
                    r: 10,
                    g: 20,
                    b: 30,
                },
                ..Default::default()
            },
        );

        let out = dim(&buf, 0.0);

        assert_eq!(
            out.get(0, 0).fg,
            Color::Rgb {
                r: 100,
                g: 150,
                b: 200
            }
        );
        assert_eq!(
            out.get(0, 0).bg,
            Color::Rgb {
                r: 10,
                g: 20,
                b: 30
            }
        );
        assert_eq!(out.get(0, 0).symbol, 'X');
    }

    #[test]
    fn dim_at_factor_one_drives_rgb_cells_to_black() {
        let mut buf = Buffer::new(1, 1);
        buf.set(
            0,
            0,
            Cell {
                fg: Color::Rgb {
                    r: 100,
                    g: 150,
                    b: 200,
                },
                bg: Color::Reset,
                ..Default::default()
            },
        );

        let out = dim(&buf, 1.0);

        assert_eq!(out.get(0, 0).fg, Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn dim_at_factor_half_halves_each_channel() {
        let mut buf = Buffer::new(1, 1);
        buf.set(
            0,
            0,
            Cell {
                fg: Color::Rgb {
                    r: 100,
                    g: 150,
                    b: 200,
                },
                bg: Color::Reset,
                ..Default::default()
            },
        );

        let out = dim(&buf, 0.5);

        assert_eq!(
            out.get(0, 0).fg,
            Color::Rgb {
                r: 50,
                g: 75,
                b: 100
            }
        );
    }

    #[test]
    fn dim_leaves_non_rgb_colors_unaffected() {
        let mut buf = Buffer::new(1, 1);
        buf.set(
            0,
            0,
            Cell {
                fg: Color::Red,
                bg: Color::Reset,
                symbol: 'Y',
                ..Default::default()
            },
        );

        let out = dim(&buf, 1.0);

        assert_eq!(out.get(0, 0).fg, Color::Red);
        assert_eq!(out.get(0, 0).bg, Color::Reset);
    }
}
