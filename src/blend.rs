//! Alpha-blending prototype for the rendering-fidelity spike
//! (docs/design/specs/core/2026-08-08-rendering-fidelity-spike-design.md).
//! Spike-only: not a committed replacement for `LayerStack::composite`'s
//! hard-cutout compositing rule.

use crate::buffer::{Buffer, Cell};
use crate::easing::lerp_color;
use crossterm::style::Color;

/// Blends `overlay`'s non-default cells over `base`, interpolating
/// fg/bg color by `alpha` (0 = base only, 1 = overlay only) via
/// `easing::lerp_color`. `overlay` cells equal to `Cell::default()`
/// are treated as "painted nothing" and skipped entirely — the same
/// transparency rule `LayerStack` already uses. The overlay's glyph
/// replaces the base's once `alpha >= 0.5` (glyphs don't blend; this
/// is a documented spike simplification). Iterates the smaller of the
/// two buffers' dimensions if they differ in size.
pub fn blend_over(base: &Buffer, overlay: &Buffer, alpha: f32) -> Buffer {
    let mut out = base.clone();
    for y in 0..base.height.min(overlay.height) {
        for x in 0..base.width.min(overlay.width) {
            let ov = overlay.get(x, y);
            if *ov == Cell::default() {
                continue;
            }
            let b = base.get(x, y);
            let blended = Cell {
                symbol: if alpha >= 0.5 { ov.symbol } else { b.symbol },
                fg: lerp_color(b.fg, ov.fg, alpha),
                bg: lerp_color(b.bg, ov.bg, alpha),
                style: if alpha >= 0.5 { ov.style } else { b.style },
            };
            out.set(x, y, blended);
        }
    }
    out
}

/// Interpolates every non-default cell's fg/bg toward `target` by
/// `factor` (0 = unchanged, 1 = fully `target`), collapsing a cell all
/// the way to `Cell::default()` once both channels are within 2 RGB
/// steps of `target` — lets a fully-faded trail cell become
/// transparent again.
///
/// **SPIKE FINDING:** this only works because `target` is `Rgb` —
/// `easing::lerp_color` falls back to its `to` argument immediately
/// for any non-`Rgb` color, so fading toward `Color::Reset` (true
/// transparency) is NOT gradual today. See this spec's
/// recommendations section (Task 9).
pub fn fade_toward(buf: &Buffer, target: Color, factor: f32) -> Buffer {
    let mut out = buf.clone();
    let close_enough = |a: Color| -> bool {
        matches!(
            (a, target),
            (
                Color::Rgb { r: r1, g: g1, b: b1 },
                Color::Rgb { r: r2, g: g2, b: b2 },
            ) if r1.abs_diff(r2) <= 2 && g1.abs_diff(g2) <= 2 && b1.abs_diff(b2) <= 2
        )
    };
    for y in 0..buf.height {
        for x in 0..buf.width {
            let c = buf.get(x, y);
            if *c == Cell::default() {
                continue;
            }
            let fg = lerp_color(c.fg, target, factor);
            let bg = lerp_color(c.bg, target, factor);
            if close_enough(fg) && close_enough(bg) {
                out.set(x, y, Cell::default());
            } else {
                out.set(
                    x,
                    y,
                    Cell {
                        fg,
                        bg,
                        ..c.clone()
                    },
                );
            }
        }
    }
    out
}
