use crossterm::style::Color;

#[derive(Clone, PartialEq, Debug)]
pub struct Cell {
    pub symbol: char,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub width: u16,
    pub height: u16,
    cells: Vec<Cell>,
}

impl Buffer {
    pub fn new(width: u16, height: u16) -> Self {
        Buffer {
            width,
            height,
            cells: vec![Cell::default(); width as usize * height as usize],
        }
    }

    pub fn get(&self, x: u16, y: u16) -> &Cell {
        &self.cells[self.index(x, y)]
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        let idx = self.index(x, y);
        self.cells[idx] = cell;
    }

    fn index(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }
}

#[derive(Debug, PartialEq)]
pub struct CellDiff {
    pub x: u16,
    pub y: u16,
    pub cell: Cell,
}

pub fn diff(prev: &Buffer, next: &Buffer) -> Vec<CellDiff> {
    let mut out = Vec::new();
    for y in 0..next.height {
        for x in 0..next.width {
            let n = next.get(x, y);
            if n != prev.get(x, y) {
                out.push(CellDiff {
                    x,
                    y,
                    cell: n.clone(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_filled_with_default_cells() {
        let buf = Buffer::new(3, 2);
        assert_eq!(buf.width, 3);
        assert_eq!(buf.height, 2);
        assert_eq!(*buf.get(0, 0), Cell::default());
        assert_eq!(*buf.get(2, 1), Cell::default());
    }

    #[test]
    fn set_then_get_returns_the_cell() {
        let mut buf = Buffer::new(3, 2);
        let cell = Cell {
            symbol: 'x',
            fg: crossterm::style::Color::Red,
            bg: crossterm::style::Color::Reset,
        };
        buf.set(1, 1, cell.clone());
        assert_eq!(*buf.get(1, 1), cell);
    }

    #[test]
    fn diff_returns_only_changed_cells() {
        let prev = Buffer::new(2, 1);
        let mut next = Buffer::new(2, 1);
        let cell = Cell {
            symbol: 'x',
            fg: Color::Reset,
            bg: Color::Reset,
        };
        next.set(1, 0, cell.clone());

        let diffs = diff(&prev, &next);

        assert_eq!(diffs, vec![CellDiff { x: 1, y: 0, cell }]);
    }

    #[test]
    fn diff_of_identical_buffers_is_empty() {
        let a = Buffer::new(2, 2);
        let b = Buffer::new(2, 2);
        assert!(diff(&a, &b).is_empty());
    }
}
