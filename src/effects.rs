//! Screen-shake and other whole-buffer visual effects.

use crate::buffer::Buffer;

/// Offsets every cell in `buf` by `(dx, dy)`, leaving cells shifted
/// off-edge blank — a one-frame screen-shake displacement.
pub fn shake(buf: &Buffer, dx: i16, dy: i16) -> Buffer {
    let mut out = Buffer::new(buf.width, buf.height);
    for y in 0..buf.height {
        for x in 0..buf.width {
            let src_x = x as i32 - dx as i32;
            let src_y = y as i32 - dy as i32;
            if src_x >= 0 && src_y >= 0 && src_x < buf.width as i32 && src_y < buf.height as i32 {
                out.set(x, y, buf.get(src_x as u16, src_y as u16).clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Cell;
    use crossterm::style::Color;

    #[test]
    fn shake_with_zero_offset_returns_unchanged_buffer() {
        let mut buf = Buffer::new(2, 2);
        let cell = Cell {
            symbol: 'x',
            fg: Color::Red,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        };
        buf.set(0, 0, cell.clone());

        let result = shake(&buf, 0, 0);

        assert_eq!(*result.get(0, 0), cell);
    }

    #[test]
    fn shake_with_positive_dx_moves_cell_right() {
        let mut buf = Buffer::new(3, 1);
        let cell = Cell {
            symbol: 'a',
            fg: Color::Green,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        };
        buf.set(0, 0, cell.clone());

        let result = shake(&buf, 1, 0);

        assert_eq!(*result.get(1, 0), cell);
        assert_eq!(*result.get(0, 0), Cell::default());
    }

    #[test]
    fn shake_with_negative_dy_moves_cell_up() {
        let mut buf = Buffer::new(1, 2);
        let cell = Cell {
            symbol: 'b',
            fg: Color::Blue,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        };
        buf.set(0, 1, cell.clone());

        let result = shake(&buf, 0, -1);

        assert_eq!(*result.get(0, 0), cell);
        assert_eq!(*result.get(0, 1), Cell::default());
    }

    #[test]
    fn shake_with_large_offset_clears_all_cells() {
        let mut buf = Buffer::new(3, 3);
        let cell = Cell {
            symbol: 'c',
            fg: Color::Cyan,
            bg: Color::Reset,
            alpha: 1.0,
            ..Default::default()
        };
        buf.set(0, 0, cell.clone());
        buf.set(1, 1, cell.clone());
        buf.set(2, 2, cell);

        let result = shake(&buf, 3, 0);

        assert_eq!(*result.get(0, 0), Cell::default());
        assert_eq!(*result.get(1, 0), Cell::default());
        assert_eq!(*result.get(2, 0), Cell::default());
        assert_eq!(*result.get(0, 1), Cell::default());
        assert_eq!(*result.get(1, 1), Cell::default());
        assert_eq!(*result.get(2, 1), Cell::default());
        assert_eq!(*result.get(0, 2), Cell::default());
        assert_eq!(*result.get(1, 2), Cell::default());
        assert_eq!(*result.get(2, 2), Cell::default());
    }
}
