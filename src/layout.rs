//! `Rect`/`Constraint`-based area splitting, in the same spirit as
//! ratatui's `Layout` — divides one area into ordered sub-areas.

/// A rectangular region in cell coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: u16,
    /// Top edge.
    pub y: u16,
    /// Width in cells.
    pub width: u16,
    /// Height in cells.
    pub height: u16,
}

impl Rect {
    /// Whether `(x, y)` falls within this rect — inclusive of the
    /// left/top edge, exclusive of the right/bottom edge (matches how
    /// `width`/`height` are already used everywhere else in this crate).
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Axis a `Layout` splits its area along.
///
/// `#[non_exhaustive]`: new variants may be added in a minor release,
/// so downstream `match`es need a wildcard arm.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    /// Split left-to-right.
    Horizontal,
    /// Split top-to-bottom.
    Vertical,
}

/// How much space one child of a `Layout` split should take.
///
/// `#[non_exhaustive]`: new variants (e.g. a content-sizing `Auto`)
/// may be added in a minor release, so downstream `match`es need a
/// wildcard arm.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Constraint {
    /// Exactly this many cells.
    Fixed(u16),
    /// This percentage of the split's total size.
    Percentage(u16),
    /// At least this many cells (currently treated as exactly this
    /// many — no growth beyond it).
    Min(u16),
    /// Share of whatever space remains after fixed/percentage/min
    /// constraints, proportional to this weight.
    Fill(u16),
}

/// Splits one `Rect` into ordered sub-`Rect`s along a `Direction`,
/// per a list of `Constraint`s.
pub struct Layout {
    direction: Direction,
    constraints: Vec<Constraint>,
    margin: u16,
    spacing: u16,
}

impl Layout {
    /// Creates a layout splitting along `direction` per `constraints`,
    /// with no margin or spacing.
    pub fn new(direction: Direction, constraints: Vec<Constraint>) -> Self {
        Layout {
            direction,
            constraints,
            margin: 0,
            spacing: 0,
        }
    }

    /// Sets a uniform margin (in cells) inset from the split area's
    /// edges before dividing it.
    pub fn margin(mut self, m: u16) -> Self {
        self.margin = m;
        self
    }

    /// Sets the gap (in cells) inserted between adjacent children.
    pub fn spacing(mut self, s: u16) -> Self {
        self.spacing = s;
        self
    }

    /// Divides `area` into one `Rect` per constraint, in order.
    pub fn split(&self, area: Rect) -> Vec<Rect> {
        let area = Rect {
            x: area.x + self.margin,
            y: area.y + self.margin,
            width: area.width.saturating_sub(self.margin * 2),
            height: area.height.saturating_sub(self.margin * 2),
        };
        let n = self.constraints.len() as u16;
        let spacing_total = if n > 0 { self.spacing * (n - 1) } else { 0 };
        let total = match self.direction {
            Direction::Horizontal => area.width,
            Direction::Vertical => area.height,
        };

        let mut sizes = vec![0u16; self.constraints.len()];
        let mut used = 0u16;
        let mut fill_indices = Vec::new();
        let mut fill_weight_total = 0u32;

        let total = total.saturating_sub(spacing_total);

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
                    sizes[i] = ((remaining as u32 * w as u32)
                        .checked_div(fill_weight_total)
                        .unwrap_or(0)) as u16;
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
            offset += size + self.spacing;
        }
        rects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit_test_rect() -> Rect {
        Rect {
            x: 5,
            y: 20,
            width: 10,
            height: 4,
        }
    }

    #[test]
    fn a_point_strictly_inside_is_contained() {
        assert!(hit_test_rect().contains(10, 22));
    }

    #[test]
    fn a_point_on_the_left_or_top_edge_is_contained() {
        assert!(hit_test_rect().contains(5, 22));
        assert!(hit_test_rect().contains(10, 20));
    }

    #[test]
    fn a_point_on_the_right_or_bottom_edge_is_not_contained() {
        assert!(!hit_test_rect().contains(15, 22));
        assert!(!hit_test_rect().contains(10, 24));
    }

    #[test]
    fn a_point_fully_outside_each_direction_is_not_contained() {
        assert!(!hit_test_rect().contains(0, 22));
        assert!(!hit_test_rect().contains(20, 22));
        assert!(!hit_test_rect().contains(10, 15));
        assert!(!hit_test_rect().contains(10, 29));
    }

    #[test]
    fn a_zero_width_or_zero_height_rect_contains_nothing() {
        let zero_w = Rect {
            x: 5,
            y: 5,
            width: 0,
            height: 10,
        };
        assert!(!zero_w.contains(5, 10));
        let zero_h = Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 0,
        };
        assert!(!zero_h.contains(10, 5));
    }

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

    #[test]
    fn margin_and_spacing_shrink_and_separate_children() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 5,
        };
        let layout = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Fixed(2), Constraint::Fixed(2)],
        )
        .margin(1)
        .spacing(1);

        let rects = layout.split(area);

        assert_eq!(
            rects,
            vec![
                Rect {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 3
                },
                Rect {
                    x: 4,
                    y: 1,
                    width: 2,
                    height: 3
                },
            ]
        );
    }
}
