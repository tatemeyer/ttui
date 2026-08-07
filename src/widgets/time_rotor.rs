use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;

pub struct TimeRotor {
    speed: f32,
}

impl TimeRotor {
    pub fn new(speed: f32) -> Self {
        TimeRotor {
            speed: speed.max(0.1),
        }
    }

    pub fn render(&self, area: Rect, tick_count: u64, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let cx = area.x + area.width / 2;
        let scaled_tick = (tick_count as f32 * self.speed) as u64;
        for row in 0..area.height {
            let h = (row as u64).wrapping_mul(374_761_393) ^ scaled_tick.wrapping_mul(668_265_263);
            let dot_pattern = (h % 256) as u32;
            let glyph = char::from_u32(0x2800 + dot_pattern).unwrap_or('\u{2800}');
            buf.set(
                cx,
                area.y + row,
                Cell {
                    symbol: glyph,
                    ..Default::default()
                },
            );
        }
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
    fn renders_one_braille_glyph_per_row_at_the_center_column() {
        let mut buf = Buffer::new(5, 4);
        TimeRotor::new(1.0).render(area(), 0, &mut buf);

        for row in 0..4 {
            assert!(is_braille(buf.get(2, row).symbol));
        }
        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn identical_inputs_render_identically() {
        let mut buf_a = Buffer::new(5, 4);
        let mut buf_b = Buffer::new(5, 4);
        TimeRotor::new(2.5).render(area(), 7, &mut buf_a);
        TimeRotor::new(2.5).render(area(), 7, &mut buf_b);

        for row in 0..4 {
            assert_eq!(buf_a.get(2, row), buf_b.get(2, row));
        }
    }

    #[test]
    fn different_speeds_render_differently_for_the_same_tick_count() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 2,
        };
        let mut slow = Buffer::new(3, 2);
        let mut fast = Buffer::new(3, 2);

        TimeRotor::new(1.0).render(area, 10, &mut slow);
        TimeRotor::new(5.0).render(area, 10, &mut fast);

        // Hand-verified for these exact inputs (area width 3, tick 10):
        // row 0's hash differs between speed 1.0 (scaled_tick=10) and
        // speed 5.0 (scaled_tick=50), so at least one cell differs.
        let mut any_different = false;
        for row in 0..2 {
            if slow.get(1, row) != fast.get(1, row) {
                any_different = true;
            }
        }
        assert!(any_different);
    }
}
