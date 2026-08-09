//! Braille-glyph rotating speed indicator — a line sweeping through
//! the area's center at an angle driven by `tick_count * speed`.

use crate::buffer::Buffer;
use crate::canvas::{Canvas, CanvasMode};
use crate::layout::Rect;
use crossterm::style::Color;

/// Radians of rotation added per `tick_count * speed` unit — tuned so
/// the sweep is visibly moving without spinning frantically at
/// typical `speed` values (roughly 0.5-5.0).
const ROTATION_RATE: f32 = 0.05;

/// A vertical rotating-speed indicator rendered as a sweeping braille
/// line through the area's center.
pub struct TimeRotor {
    speed: f32,
}

impl TimeRotor {
    /// Creates a rotor at `speed` (floored at `0.1` so it never fully
    /// stops).
    pub fn new(speed: f32) -> Self {
        TimeRotor {
            speed: speed.max(0.1),
        }
    }

    /// Renders a line sweeping through `area`'s center, its angle
    /// driven by `tick_count` and `speed`.
    pub fn render(&self, area: Rect, tick_count: u64, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let grid_w = (area.width * 2) as f32;
        let grid_h = (area.height * 4) as f32;
        let cx = grid_w / 2.0;
        let cy = grid_h / 2.0;
        let radius = cx.min(cy).max(1.0);
        let angle = tick_count as f32 * self.speed * ROTATION_RATE;
        let (dx, dy) = (angle.cos() * radius, angle.sin() * radius);
        canvas.line(
            (cx - dx).round() as u16,
            (cy - dy).round() as u16,
            (cx + dx).round() as u16,
            (cy + dy).round() as u16,
            Color::Reset,
        );
        canvas.blit(buf, area.x, area.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 4,
        }
    }

    fn is_braille(ch: char) -> bool {
        ('\u{2800}'..='\u{28FF}').contains(&ch)
    }

    #[test]
    fn renders_at_least_one_braille_glyph_in_the_area() {
        let mut buf = Buffer::new(5, 4);
        TimeRotor::new(1.0).render(area(), 0, &mut buf);
        let mut found = false;
        for y in 0..4 {
            for x in 0..5 {
                if is_braille(buf.get(x, y).symbol) {
                    found = true;
                }
            }
        }
        assert!(found, "expected at least one braille glyph drawn");
    }

    #[test]
    fn identical_inputs_render_identically() {
        let mut buf_a = Buffer::new(5, 4);
        let mut buf_b = Buffer::new(5, 4);
        TimeRotor::new(2.5).render(area(), 7, &mut buf_a);
        TimeRotor::new(2.5).render(area(), 7, &mut buf_b);
        for y in 0..4 {
            for x in 0..5 {
                assert_eq!(buf_a.get(x, y), buf_b.get(x, y));
            }
        }
    }

    #[test]
    fn different_speeds_render_differently_for_the_same_tick_count() {
        let mut slow = Buffer::new(5, 4);
        let mut fast = Buffer::new(5, 4);
        TimeRotor::new(1.0).render(area(), 10, &mut slow);
        TimeRotor::new(8.0).render(area(), 10, &mut fast);
        let mut any_different = false;
        for y in 0..4 {
            for x in 0..5 {
                if slow.get(x, y) != fast.get(x, y) {
                    any_different = true;
                }
            }
        }
        assert!(
            any_different,
            "expected a visibly different rotation angle between speed 1.0 and 8.0 at tick 10"
        );
    }

    #[test]
    fn zero_size_area_does_not_panic() {
        let mut buf = Buffer::new(1, 1);
        let area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        };
        TimeRotor::new(1.0).render(area, 0, &mut buf);
    }
}
