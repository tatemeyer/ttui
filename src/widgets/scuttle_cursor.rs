use crate::buffer::{Buffer, Cell};

pub struct ScuttleCursor {
    symbol: char,
}

impl ScuttleCursor {
    pub fn new(symbol: char) -> Self {
        ScuttleCursor { symbol }
    }

    pub fn render(&self, x: f32, y: f32, moving: bool, tick_count: u64, buf: &mut Buffer) {
        let jerk: i32 = if moving {
            if tick_count.is_multiple_of(2) {
                -1
            } else {
                1
            }
        } else {
            0
        };
        let px = x.round() as i32 + jerk;
        let py = y.round() as i32;
        if px >= 0 && py >= 0 && (px as u16) < buf.width && (py as u16) < buf.height {
            buf.set(
                px as u16,
                py as u16,
                Cell {
                    symbol: self.symbol,
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
    fn idle_cursor_ignores_tick_count() {
        let mut buf_a = Buffer::new(10, 5);
        ScuttleCursor::new('C').render(3.4, 2.0, false, 0, &mut buf_a);
        assert_eq!(buf_a.get(3, 2).symbol, 'C');

        let mut buf_b = Buffer::new(10, 5);
        ScuttleCursor::new('C').render(3.4, 2.0, false, 99, &mut buf_b);
        assert_eq!(buf_b.get(3, 2).symbol, 'C');
    }

    #[test]
    fn moving_cursor_shifts_left_on_even_tick() {
        let mut buf = Buffer::new(10, 5);
        ScuttleCursor::new('C').render(5.0, 2.0, true, 0, &mut buf);
        assert_eq!(buf.get(4, 2).symbol, 'C');
        assert_eq!(buf.get(5, 2).symbol, ' ');
    }

    #[test]
    fn moving_cursor_shifts_right_on_odd_tick() {
        let mut buf = Buffer::new(10, 5);
        ScuttleCursor::new('C').render(5.0, 2.0, true, 1, &mut buf);
        assert_eq!(buf.get(6, 2).symbol, 'C');
        assert_eq!(buf.get(5, 2).symbol, ' ');
    }

    #[test]
    fn jerked_position_outside_bounds_does_not_panic() {
        let mut buf = Buffer::new(3, 3);
        ScuttleCursor::new('C').render(0.0, 0.0, true, 0, &mut buf);
        assert_eq!(*buf.get(0, 0), Cell::default());
    }
}
