#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Constraint {
    Fixed(u16),
    Percentage(u16),
    Min(u16),
    Fill(u16),
}

#[allow(dead_code)]
pub struct Layout {
    direction: Direction,
    constraints: Vec<Constraint>,
    margin: u16,
    spacing: u16,
}

impl Layout {
    pub fn new(direction: Direction, constraints: Vec<Constraint>) -> Self {
        Layout {
            direction,
            constraints,
            margin: 0,
            spacing: 0,
        }
    }

    pub fn split(&self, area: Rect) -> Vec<Rect> {
        let total = match self.direction {
            Direction::Horizontal => area.width,
            Direction::Vertical => area.height,
        };

        let mut sizes = vec![0u16; self.constraints.len()];
        let mut used = 0u16;
        let mut fill_indices = Vec::new();
        let mut fill_weight_total = 0u32;

        for (i, c) in self.constraints.iter().enumerate() {
            match c {
                Constraint::Fixed(v) => {
                    sizes[i] = *v;
                    used += v;
                }
                Constraint::Percentage(p) => {
                    let v = (total as u32 * *p as u32 / 100) as u16;
                    sizes[i] = v;
                    used += v;
                }
                Constraint::Min(v) => {
                    sizes[i] = *v;
                    used += v;
                }
                Constraint::Fill(w) => {
                    fill_indices.push(i);
                    fill_weight_total += *w as u32;
                }
            }
        }

        let remaining = total.saturating_sub(used);
        if fill_weight_total > 0 {
            for &i in &fill_indices {
                if let Constraint::Fill(w) = self.constraints[i] {
                    sizes[i] = (remaining as u32 * w as u32 / fill_weight_total) as u16;
                }
            }
        }

        let mut rects = Vec::with_capacity(sizes.len());
        let mut offset = 0u16;
        for &size in &sizes {
            let rect = match self.direction {
                Direction::Horizontal => Rect {
                    x: area.x + offset,
                    y: area.y,
                    width: size,
                    height: area.height,
                },
                Direction::Vertical => Rect {
                    x: area.x,
                    y: area.y + offset,
                    width: area.width,
                    height: size,
                },
            };
            rects.push(rect);
            offset += size;
        }
        rects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_constraints_split_horizontally() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 5,
        };
        let layout = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Fixed(3), Constraint::Fixed(7)],
        );

        let rects = layout.split(area);

        assert_eq!(
            rects,
            vec![
                Rect {
                    x: 0,
                    y: 0,
                    width: 3,
                    height: 5
                },
                Rect {
                    x: 3,
                    y: 0,
                    width: 7,
                    height: 5
                },
            ]
        );
    }

    #[test]
    fn percentage_constraints_split_vertically() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 10,
        };
        let layout = Layout::new(
            Direction::Vertical,
            vec![Constraint::Percentage(40), Constraint::Percentage(60)],
        );

        let rects = layout.split(area);

        assert_eq!(
            rects,
            vec![
                Rect {
                    x: 0,
                    y: 0,
                    width: 4,
                    height: 4
                },
                Rect {
                    x: 0,
                    y: 4,
                    width: 4,
                    height: 6
                },
            ]
        );
    }

    #[test]
    fn fill_constraints_split_remaining_space_by_weight() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        };
        let layout = Layout::new(
            Direction::Horizontal,
            vec![
                Constraint::Fixed(4),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ],
        );

        let rects = layout.split(area);

        assert_eq!(
            rects,
            vec![
                Rect {
                    x: 0,
                    y: 0,
                    width: 4,
                    height: 1
                },
                Rect {
                    x: 4,
                    y: 0,
                    width: 3,
                    height: 1
                },
                Rect {
                    x: 7,
                    y: 0,
                    width: 3,
                    height: 1
                },
            ]
        );
    }
}
