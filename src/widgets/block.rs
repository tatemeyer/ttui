//! Bordered container, with an optional outward second border ring
//! for a "glow" look when `Theme.border_thick` is set.
//!
//! `Block` draws chrome and hands back the space inside it; it is not a
//! combinator that takes a child widget. `render` returns the inner
//! `Rect` and the caller renders into that, which keeps `Block` the same
//! stateless `(data, area) -> paint calls` shape as every other widget
//! rather than making one of them generic over its content (#113).

use crate::buffer::{Buffer, Cell, CellStyle};
use crate::layout::Rect;
use crate::theme::{BorderSet, Theme};
use crossterm::style::Color;

/// A bordered box with an optional title, drawn with a `Theme` or
/// plain default styling.
pub struct Block<'a> {
    title: Option<&'a str>,
    theme: Option<&'a Theme>,
}

impl<'a> Block<'a> {
    /// Creates an untitled, unthemed block.
    pub fn new() -> Self {
        Block {
            title: None,
            theme: None,
        }
    }

    /// Sets the title shown on the top border.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    /// Sets the theme controlling border glyphs/color/thickness.
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Draws the border (and title, if set) into `area` and returns the
    /// inner content area for the caller to render into. The returned
    /// `Rect` is `area` inset by one cell on every side; `border_thick`
    /// draws its second ring *outward*, so it does not shrink this.
    pub fn render(&self, area: Rect, buf: &mut Buffer) -> Rect {
        if area.width < 2 || area.height < 2 {
            return area;
        }
        let (border, fg, bg, border_style, border_thick, primary_end) = match self.theme {
            Some(t) => (
                t.border,
                t.primary,
                t.background,
                t.border_style,
                t.border_thick,
                t.primary_end,
            ),
            None => (
                BorderSet::default(),
                Color::Reset,
                Color::Reset,
                CellStyle::default(),
                false,
                None,
            ),
        };
        let ring_fg = |x: u16, y: u16| -> Color {
            match primary_end {
                Some(end) => {
                    let t = ((x as f32 - area.x as f32) / area.width.max(1) as f32
                        + (y as f32 - area.y as f32) / area.height.max(1) as f32)
                        .clamp(0.0, 1.0);
                    crate::easing::lerp_color(fg, end, t)
                }
                None => fg,
            }
        };
        // The struct-update base every border/title cell is built from:
        // callers override only `symbol` (and `style`, for the title).
        let base_cell = |x: u16, y: u16| Cell {
            symbol: ' ',
            fg: ring_fg(x, y),
            bg,
            style: border_style,
            alpha: 1.0,
        };

        let draw_ring = |ring_area: Rect, buf: &mut Buffer| {
            for x in ring_area.x..ring_area.x + ring_area.width {
                buf.set(
                    x,
                    ring_area.y,
                    Cell {
                        symbol: border.horizontal,
                        ..base_cell(x, ring_area.y)
                    },
                );
                buf.set(
                    x,
                    ring_area.y + ring_area.height - 1,
                    Cell {
                        symbol: border.horizontal,
                        ..base_cell(x, ring_area.y + ring_area.height - 1)
                    },
                );
            }
            for y in ring_area.y..ring_area.y + ring_area.height {
                buf.set(
                    ring_area.x,
                    y,
                    Cell {
                        symbol: border.vertical,
                        ..base_cell(ring_area.x, y)
                    },
                );
                buf.set(
                    ring_area.x + ring_area.width - 1,
                    y,
                    Cell {
                        symbol: border.vertical,
                        ..base_cell(ring_area.x + ring_area.width - 1, y)
                    },
                );
            }
            buf.set(
                ring_area.x,
                ring_area.y,
                Cell {
                    symbol: border.top_left,
                    ..base_cell(ring_area.x, ring_area.y)
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                ring_area.y,
                Cell {
                    symbol: border.top_right,
                    ..base_cell(ring_area.x + ring_area.width - 1, ring_area.y)
                },
            );
            buf.set(
                ring_area.x,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.bottom_left,
                    ..base_cell(ring_area.x, ring_area.y + ring_area.height - 1)
                },
            );
            buf.set(
                ring_area.x + ring_area.width - 1,
                ring_area.y + ring_area.height - 1,
                Cell {
                    symbol: border.bottom_right,
                    ..base_cell(
                        ring_area.x + ring_area.width - 1,
                        ring_area.y + ring_area.height - 1,
                    )
                },
            );
        };

        draw_ring(area, buf);

        if border_thick {
            let outer_x = area.x.saturating_sub(1);
            let outer_y = area.y.saturating_sub(1);
            let outer_w = (area.width + 2).min(buf.width.saturating_sub(outer_x));
            let outer_h = (area.height + 2).min(buf.height.saturating_sub(outer_y));
            if outer_w >= 2 && outer_h >= 2 {
                draw_ring(
                    Rect {
                        x: outer_x,
                        y: outer_y,
                        width: outer_w,
                        height: outer_h,
                    },
                    buf,
                );
            }
        }

        if let Some(title) = self.title {
            for (i, ch) in title
                .chars()
                .take(area.width.saturating_sub(2) as usize)
                .enumerate()
            {
                let x = area.x + 1 + i as u16;
                buf.set(
                    x,
                    area.y,
                    Cell {
                        symbol: ch,
                        // Load-bearing: without this override the title
                        // silently inherits the theme's `border_style`
                        // (bold borders would bold the title too). Deleting
                        // it still compiles — `title_cells_are_not_bold_
                        // even_when_theme_border_style_is_bold` is what
                        // catches it.
                        style: CellStyle::default(),
                        ..base_cell(x, area.y)
                    },
                );
            }
        }

        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    }
}

impl<'a> Default for Block<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Buffer, Intensity};
    use crate::layout::Rect;
    use crate::theme::{BorderSet, Theme};

    #[test]
    fn draws_border_and_returns_inner_area() {
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        let inner = Block::new().render(area, &mut buf);

        assert_eq!(
            inner,
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 1
            }
        );
        assert_eq!(buf.get(0, 0).symbol, '┌');
        assert_eq!(buf.get(1, 0).symbol, '─');
        assert_eq!(buf.get(0, 1).symbol, '│');
    }

    #[test]
    fn title_is_drawn_on_the_top_border() {
        let mut buf = Buffer::new(6, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 6,
            height: 3,
        };

        Block::new().title("Hi").render(area, &mut buf);

        assert_eq!(buf.get(1, 0).symbol, 'H');
        assert_eq!(buf.get(2, 0).symbol, 'i');
    }

    #[test]
    fn without_theme_border_colors_are_reset() {
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '┌');
        assert_eq!(buf.get(0, 0).fg, Color::Reset);
        assert_eq!(buf.get(0, 0).bg, Color::Reset);
        assert_eq!(buf.get(0, 0).style.intensity, Intensity::Normal);
    }

    #[test]
    fn with_theme_border_uses_theme_glyphs_and_colors() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle::default(),
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '*'); // corner
        assert_eq!(buf.get(1, 0).symbol, '='); // horizontal
        assert_eq!(buf.get(0, 1).symbol, '#'); // vertical
        assert_eq!(buf.get(0, 0).fg, Color::Green);
        assert_eq!(buf.get(0, 0).bg, Color::Black);
    }

    #[test]
    fn border_cells_are_bold_when_theme_border_style_is_bold() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle {
                intensity: Intensity::Bold,
                ..Default::default()
            },
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).style.intensity, Intensity::Bold); // corner
        assert_eq!(buf.get(1, 0).style.intensity, Intensity::Bold); // horizontal edge
        assert_eq!(buf.get(0, 1).style.intensity, Intensity::Bold); // vertical edge
    }

    #[test]
    fn border_cells_carry_arbitrary_cellstyle_fields_not_just_intensity() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle {
                underline: true,
                ..Default::default()
            },
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert!(buf.get(0, 0).style.underline); // corner
        assert!(buf.get(1, 0).style.underline); // horizontal edge
        assert!(buf.get(0, 1).style.underline); // vertical edge
    }

    #[test]
    fn thick_border_draws_a_second_ring_one_cell_outward() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle::default(),
            border_thick: true,
        };
        let mut buf = Buffer::new(6, 5);
        let area = Rect {
            x: 1,
            y: 1,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '*'); // outer corner
        assert_eq!(buf.get(1, 0).symbol, '='); // outer top edge
        assert_eq!(buf.get(0, 1).symbol, '#'); // outer left edge
        assert_eq!(buf.get(0, 0).fg, Color::Green);
        assert_eq!(buf.get(0, 0).bg, Color::Black);
    }

    #[test]
    fn thin_border_leaves_the_outward_ring_untouched() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle::default(),
            border_thick: false,
        };
        let mut buf = Buffer::new(6, 5);
        let area = Rect {
            x: 1,
            y: 1,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn theme_less_border_leaves_the_outward_ring_untouched() {
        let mut buf = Buffer::new(6, 5);
        let area = Rect {
            x: 1,
            y: 1,
            width: 4,
            height: 3,
        };

        Block::new().render(area, &mut buf);

        assert_eq!(*buf.get(0, 0), Cell::default());
    }

    #[test]
    fn title_cells_are_not_bold_even_when_theme_border_style_is_bold() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle {
                intensity: Intensity::Bold,
                ..Default::default()
            },
            border_thick: false,
        };
        let mut buf = Buffer::new(6, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 6,
            height: 3,
        };

        Block::new()
            .title("Hi")
            .theme(&theme)
            .render(area, &mut buf);

        assert_eq!(buf.get(1, 0).style.intensity, Intensity::Normal); // 'H'
        assert_eq!(buf.get(2, 0).style.intensity, Intensity::Normal); // 'i'

        // Both facts in one render: the same theme that leaves the title
        // unbold above does bold the border here. Asserting only the
        // title half leaves "the theme was actually applied" to be
        // inferred from a separate test.
        assert_eq!(buf.get(0, 0).style.intensity, Intensity::Bold); // corner
        assert_eq!(buf.get(3, 0).style.intensity, Intensity::Bold); // edge past the title
        assert_eq!(buf.get(0, 1).style.intensity, Intensity::Bold); // vertical edge
    }

    #[test]
    fn primary_end_none_produces_byte_for_byte_identical_output_to_flat_color() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle::default(),
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        // Every border cell must be flat theme.primary — the exact
        // regression guarantee for existing themed apps that never
        // set primary_end.
        for x in 0..4 {
            assert_eq!(buf.get(x, 0).fg, Color::Green);
            assert_eq!(buf.get(x, 2).fg, Color::Green);
        }
        for y in 0..3 {
            assert_eq!(buf.get(0, y).fg, Color::Green);
            assert_eq!(buf.get(3, y).fg, Color::Green);
        }
    }

    #[test]
    fn primary_end_some_lerps_color_across_the_border_ring() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Rgb { r: 0, g: 0, b: 0 },
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: Some(Color::Rgb {
                r: 200,
                g: 100,
                b: 50,
            }),
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle::default(),
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        // Top-left corner (0,0) is at perimeter position t=0 -> exactly primary.
        assert_eq!(buf.get(0, 0).fg, Color::Rgb { r: 0, g: 0, b: 0 });
        // Bottom-right corner (3,2) is at perimeter position t=1 (clamped) -> exactly primary_end.
        assert_eq!(
            buf.get(3, 2).fg,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50
            }
        );
        // A cell strictly between the two corners must differ from both endpoints.
        let mid = buf.get(3, 0).fg;
        assert_ne!(mid, Color::Rgb { r: 0, g: 0, b: 0 });
        assert_ne!(
            mid,
            Color::Rgb {
                r: 200,
                g: 100,
                b: 50
            }
        );
    }

    #[test]
    fn non_rgb_primary_with_primary_end_steps_between_the_two_endpoint_colors() {
        // Pins easing::lerp_color's fallback for a pair it cannot
        // interpolate: rather than inventing an RGB value for a named
        // color the terminal may render differently, it steps from
        // `primary` to `primary_end` at the gradient's midpoint. The
        // endpoints are therefore honest even though the ramp is not
        // smooth (#122).
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: Some(Color::Red),
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '*',
                top_right: '*',
                bottom_left: '*',
                bottom_right: '*',
            },
            border_style: CellStyle::default(),
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        // The near corner is the gradient's t=0 end, so it must show
        // `primary`. Before #122 this was `primary_end` too — the whole
        // ring rendered flat at the end color and the start color never
        // appeared at all.
        assert_eq!(
            buf.get(0, 0).fg,
            Color::Green,
            "the gradient's start corner must show `primary`"
        );
        // The far corner is t=1 and must show `primary_end`.
        assert_eq!(
            buf.get(3, 2).fg,
            Color::Red,
            "the gradient's end corner must show `primary_end`"
        );
        // Every *border* cell is one of the two endpoint colors — a
        // non-interpolable pair steps between them rather than ramping,
        // so no third, invented shade is ever emitted. (Interior cells
        // are untouched `Cell::default()`s and not part of the ring.)
        for (x, y) in (0..4)
            .flat_map(|x| (0..3).map(move |y| (x, y)))
            .filter(|&(x, y)| x == 0 || x == 3 || y == 0 || y == 2)
        {
            let fg = buf.get(x, y).fg;
            assert!(
                fg == Color::Green || fg == Color::Red,
                "border cell ({x},{y}) rendered {fg:?}, which is neither endpoint"
            );
        }
    }

    #[test]
    fn all_four_corners_render_their_own_distinct_glyph() {
        let theme = Theme {
            background: Color::Black,
            primary: Color::Green,
            secondary: Color::Reset,
            tertiary: Color::Reset,
            accent: Color::Reset,
            primary_end: None,
            border: BorderSet {
                horizontal: '=',
                vertical: '#',
                top_left: '1',
                top_right: '2',
                bottom_left: '3',
                bottom_right: '4',
            },
            border_style: CellStyle::default(),
            border_thick: false,
        };
        let mut buf = Buffer::new(4, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 3,
        };

        Block::new().theme(&theme).render(area, &mut buf);

        assert_eq!(buf.get(0, 0).symbol, '1'); // top-left
        assert_eq!(buf.get(3, 0).symbol, '2'); // top-right
        assert_eq!(buf.get(0, 2).symbol, '3'); // bottom-left
        assert_eq!(buf.get(3, 2).symbol, '4'); // bottom-right
    }
}
