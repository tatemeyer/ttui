//! Circular item-select dial: items placed evenly around an ellipse,
//! the selected one highlighted.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crossterm::style::Color;

/// A ring of selectable items arranged around an ellipse.
pub struct Dial<'a> {
    items: &'a [String],
    selected: usize,
}

impl<'a> Dial<'a> {
    /// Creates a dial over `items`, highlighting the one at `selected`.
    pub fn new(items: &'a [String], selected: usize) -> Self {
        Dial { items, selected }
    }

    /// Renders every item around the dial, centered in `area`.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let n = self.items.len();
        if n == 0 || area.width == 0 || area.height == 0 {
            return;
        }

        let cx = area.x as f32 + area.width as f32 / 2.0;
        let cy = area.y as f32 + area.height as f32 / 2.0;
        let radius_y = ((area.height as i32 / 2 - 1).max(1)) as f32;
        let radius_x = (((radius_y as i32) * 2)
            .min(area.width as i32 / 2 - 1)
            .max(1)) as f32;

        let angle_of = |i: usize| -> f32 {
            i as f32 * std::f32::consts::TAU / n as f32 - std::f32::consts::FRAC_PI_2
        };
        let point_at = |angle: f32, rx: f32, ry: f32| -> (f32, f32) {
            (cx + rx * angle.cos(), cy + ry * angle.sin())
        };

        for i in 0..n {
            let a0 = angle_of(i);
            let a1 = angle_of((i + 1) % n)
                + if i + 1 == n {
                    std::f32::consts::TAU
                } else {
                    0.0
                };
            for step in 1..4 {
                let t = step as f32 / 4.0;
                let angle = a0 + (a1 - a0) * t;
                let (x, y) = point_at(angle, radius_x, radius_y);
                let (px, py) = (x.round() as i32, y.round() as i32);
                if in_area(px, py, area) {
                    buf.set(
                        px as u16,
                        py as u16,
                        Cell {
                            symbol: '.',
                            ..Default::default()
                        },
                    );
                }
            }
        }

        for (i, item) in self.items.iter().enumerate() {
            let angle = angle_of(i);
            let (x, y) = point_at(angle, radius_x, radius_y);
            let (px, py) = (x.round() as i32, y.round() as i32);
            let selected = i == self.selected;
            let (fg, bg) = if selected {
                (Color::Black, Color::White)
            } else {
                (Color::Reset, Color::Reset)
            };

            let chars: Vec<char> = item.chars().collect();
            if x >= cx {
                for (offset, ch) in chars.iter().enumerate() {
                    let cell_x = px + offset as i32;
                    if in_area(cell_x, py, area) {
                        buf.set(
                            cell_x as u16,
                            py as u16,
                            Cell {
                                symbol: *ch,
                                fg,
                                bg,
                                ..Default::default()
                            },
                        );
                    }
                }
            } else {
                let len = chars.len() as i32;
                for (offset, ch) in chars.iter().enumerate() {
                    let cell_x = px - (len - 1 - offset as i32);
                    if in_area(cell_x, py, area) {
                        buf.set(
                            cell_x as u16,
                            py as u16,
                            Cell {
                                symbol: *ch,
                                fg,
                                bg,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            if selected {
                let (px2, py2) = point_at(angle, radius_x * 0.5, radius_y * 0.5);
                let (ppx, ppy) = (px2.round() as i32, py2.round() as i32);
                if in_area(ppx, ppy, area) {
                    buf.set(
                        ppx as u16,
                        ppy as u16,
                        Cell {
                            symbol: '*',
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

fn in_area(x: i32, y: i32, area: Rect) -> bool {
    x >= area.x as i32
        && x < area.x as i32 + area.width as i32
        && y >= area.y as i32
        && y < area.y as i32 + area.height as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_zero_lands_at_top_center_column() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut buf = Buffer::new(10, 8);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(5, 1).symbol, 'A');
    }

    #[test]
    fn items_are_symmetric_left_and_right_for_odd_item_count() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut buf = Buffer::new(10, 8);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(8, 6).symbol, 'B');
        assert_eq!(buf.get(2, 6).symbol, 'C');
    }

    #[test]
    fn ring_dots_never_land_on_an_item_point() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut buf = Buffer::new(10, 8);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(7, 1).symbol, '.');
        assert_eq!(buf.get(8, 3).symbol, '.');
        assert_eq!(buf.get(9, 4).symbol, '.');
    }

    #[test]
    fn selected_items_label_and_pointer_are_highlighted() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut buf = Buffer::new(10, 8);
        let area = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 8,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        assert_eq!(buf.get(5, 1).fg, Color::Black);
        assert_eq!(buf.get(5, 1).bg, Color::White);
        assert_eq!(buf.get(5, 3).symbol, '*');
        assert_eq!(buf.get(8, 6).bg, Color::Reset);
    }

    #[test]
    fn labels_flow_outward_and_clip_at_area_edges() {
        let items = vec![
            "TOP".to_string(),
            "RIGHT".to_string(),
            "BOTTOM".to_string(),
            "LEFT".to_string(),
        ];
        let mut buf = Buffer::new(6, 4);
        let area = Rect {
            x: 0,
            y: 0,
            width: 6,
            height: 4,
        };

        Dial::new(&items, 0).render(area, &mut buf);

        // item1 "RIGHT" starts at column 5 flowing right; the area is
        // only 6 columns wide (0..5), so only 'R' fits.
        assert_eq!(buf.get(5, 2).symbol, 'R');
        // item3 "LEFT" ends at column 1 flowing left; only 'F' and 'T'
        // fit before the area's left edge clips the rest.
        assert_eq!(buf.get(0, 2).symbol, 'F');
        assert_eq!(buf.get(1, 2).symbol, 'T');
    }
}
