//! Decaying noise overlay for glitch/corruption effects — shared
//! across screens that render mutually-exclusive glitch states.

use crate::buffer::{Buffer, Cell};
use crate::layout::Rect;
use crate::transition::Transition;
use crossterm::style::Color;
use std::time::Duration;

const GLYPHS: [char; 4] = ['░', '▒', '▓', '█'];

/// A decaying block-glyph noise overlay for glitch/corruption effects
/// — trigger it, tick it each frame, render it while active.
pub struct GlitchBuffer {
    transition: Option<Transition>,
    alpha: f32,
}

impl GlitchBuffer {
    /// Creates an inactive `GlitchBuffer`.
    pub fn new() -> Self {
        GlitchBuffer {
            transition: None,
            alpha: 1.0,
        }
    }

    /// Sets the alpha every rendered glitch cell carries, for a
    /// partially-transparent effect (e.g. "static laid over the
    /// readout, not fully opaque"). Expected range is `0.0`-`1.0`,
    /// matching `Cell.alpha`'s own documented range — values outside
    /// it are not clamped or validated (caller error, same as passing
    /// an out-of-range alpha to `Cell` directly). Defaults to `1.0`
    /// (fully opaque) — existing callers that never call this see no
    /// behavior change.
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    /// Starts (or restarts) the glitch, decaying over `duration`.
    pub fn trigger(&mut self, duration: Duration) {
        self.transition = Some(Transition::start(duration));
    }

    /// Advances the decay by `elapsed`; deactivates once complete.
    pub fn tick(&mut self, elapsed: Duration) {
        if let Some(t) = &mut self.transition {
            t.tick(elapsed);
            if t.is_complete() {
                self.transition = None;
            }
        }
    }

    /// Ends the glitch immediately, regardless of remaining duration.
    pub fn clear(&mut self) {
        self.transition = None;
    }

    /// Whether the glitch is currently decaying (i.e. should render).
    pub fn is_active(&self) -> bool {
        self.transition.is_some()
    }

    /// Overlays deterministic noise glyphs in `color` across `area`,
    /// density scaling with remaining intensity. A no-op when inactive.
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
                            alpha: self.alpha,
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
    fn clear_deactivates_immediately_regardless_of_remaining_duration() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(600));
        assert!(gb.is_active());
        gb.clear();
        assert!(!gb.is_active());
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

    #[test]
    fn default_alpha_is_1_0() {
        let mut gb = GlitchBuffer::new();
        gb.trigger(Duration::from_millis(500));
        let mut buf = Buffer::new(3, 3);

        gb.render(area(), Color::Red, 0, &mut buf);

        assert_eq!(buf.get(1, 1).alpha, 1.0);
    }

    #[test]
    fn with_alpha_sets_every_rendered_cells_alpha() {
        let mut gb = GlitchBuffer::new().with_alpha(0.5);
        gb.trigger(Duration::from_millis(500));
        let mut buf = Buffer::new(3, 3);

        gb.render(area(), Color::Red, 0, &mut buf);

        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(buf.get(x, y).alpha, 0.5);
            }
        }
    }
}
