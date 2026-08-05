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

pub struct LayerStack {
    width: u16,
    height: u16,
    layers: Vec<Buffer>,
}

impl LayerStack {
    pub fn new(width: u16, height: u16) -> Self {
        LayerStack {
            width,
            height,
            layers: vec![Buffer::new(width, height)],
        }
    }

    pub fn push_layer(&mut self) -> &mut Buffer {
        self.layers.push(Buffer::new(self.width, self.height));
        self.layers.last_mut().unwrap()
    }

    pub fn layer_mut(&mut self, index: usize) -> &mut Buffer {
        &mut self.layers[index]
    }
}

impl std::ops::Deref for LayerStack {
    type Target = Buffer;
    fn deref(&self) -> &Buffer {
        &self.layers[0]
    }
}

impl std::ops::DerefMut for LayerStack {
    fn deref_mut(&mut self) -> &mut Buffer {
        &mut self.layers[0]
    }
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

    #[test]
    fn new_layer_stack_has_one_default_filled_base_layer() {
        let mut stack = LayerStack::new(3, 2);
        assert_eq!(*stack.layer_mut(0).get(0, 0), Cell::default());
        assert_eq!(*stack.layer_mut(0).get(2, 1), Cell::default());
    }

    #[test]
    fn push_layer_appends_a_same_dimension_default_filled_layer() {
        let mut stack = LayerStack::new(3, 2);
        let cell = Cell {
            symbol: 'x',
            fg: Color::Red,
            bg: Color::Reset,
        };
        stack.push_layer().set(1, 1, cell.clone());

        assert_eq!(*stack.layer_mut(1).get(1, 1), cell);
        assert_eq!(*stack.layer_mut(0).get(1, 1), Cell::default());
    }

    #[test]
    fn layer_stack_derefs_to_the_base_layer() {
        let mut stack = LayerStack::new(3, 2);
        let cell = Cell {
            symbol: 'y',
            fg: Color::Reset,
            bg: Color::Red,
        };
        stack.set(0, 1, cell.clone()); // DerefMut -> base layer, no layer_mut(0) needed

        assert_eq!(*stack.get(0, 1), cell); // Deref -> base layer
        assert_eq!(*stack.layer_mut(0).get(0, 1), cell); // same cell via explicit index
    }
}
