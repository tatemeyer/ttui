use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::transition::Transition;
use crossterm::style::Color;
use std::time::Duration;

const GLYPHS: [char; 4] = ['░', '▒', '▓', '█'];

pub struct GlitchBuffer {
    transition: Option<Transition>,
}

impl GlitchBuffer {
    pub fn new() -> Self {
        GlitchBuffer { transition: None }
    }

    pub fn trigger(&mut self, duration: Duration) {
        self.transition = Some(Transition::start(duration));
    }

    pub fn tick(&mut self, elapsed: Duration) {
        if let Some(t) = &mut self.transition {
            t.tick(elapsed);
            if t.is_complete() {
                self.transition = None;
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.transition.is_some()
    }

    pub fn render(&self, area: Rect, color: Color, tick_count: u64, buf: &mut Buffer) {
        let Some(t) = &self.transition else { return };
        let intensity = 1.0 - t.progress();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let h = (x as u64).wrapping_mul(374_761_393)
                    ^ (y as u64).wrapping_mul(668_265_263)
                    ^ tick_count.wrapping_mul(2_246_822_519);
                let roll = (h % 1000) as f32 / 1000.0;
                if roll < intensity {
                    let glyph = GLYPHS[(h / 1000 % 4) as usize];
                    buf.set(
                        x,
                        y,
                        Cell {
                            symbol: glyph,
                            fg: color,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

impl Default for GlitchBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        }
    }

    #[test]
    fn fresh_glitch_buffer_is_inactive_and_render_is_a_no_op() {
        let gb = GlitchBuffer::new();
        let mut buf = Buffer::new(3, 3);

        assert!(!gb.is_active());
        gb.render(area(), Color::Red, 0, &mut buf);

        assert_eq!(*buf.get(1, 1), Cell::default());
    }

    #[test]
    fn trigger_makes_is_active_true() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(500));
        assert!(gb.is_active());
    }

    #[test]
    fn ticking_past_the_triggered_duration_deactivates_it() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(500));
        gb.tick(Duration::from_millis(600));
        assert!(!gb.is_active());

        let mut buf = Buffer::new(3, 3);
        gb.render(area(), Color::Red, 0, &mut buf);
        assert_eq!(*buf.get(1, 1), Cell::default());
    }

    #[test]
    fn at_full_intensity_every_cell_is_glitched_with_the_requested_color() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(500));
        let mut buf = Buffer::new(3, 3);

        gb.render(area(), Color::Red, 0, &mut buf);

        for y in 0..3 {
            for x in 0..3 {
                let cell = buf.get(x, y);
                assert_ne!(*cell, Cell::default());
                assert_eq!(cell.fg, Color::Red);
                assert!(GLYPHS.contains(&cell.symbol));
            }
        }
    }
}
