//! Rasterizes a parsed `vt100::Screen` into an RGBA bitmap: one 16x16
//! pixel block per terminal cell, built from 2x-upscaled 8x8 glyphs.
//! Approximates bold/reverse/underline; deliberately does not attempt
//! italic (unrenderable in a fixed bitmap font) or dim/strikethrough
//! (not exposed by `vt100::Cell`).

use crate::{color, glyph};
use image::{Rgba, RgbaImage};

const CELL_PX: u32 = 16;
const GLYPH_PX: u32 = 8;
const SCALE: u32 = CELL_PX / GLYPH_PX;

/// A codepoint at a given (row, col) that `glyph::glyph_for` could not map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    /// The unmapped-glyph error, plus the cell position that hit it.
    Glyph(glyph::GlyphError, u16, u16),
}

/// `color::to_rgb` is context-free: for `vt100::Color::Default` it always
/// returns the default *foreground* shade (light gray), since it has no
/// way to know whether its caller means fg or bg. The ANSI convention —
/// default background is black — has to be applied by the caller for
/// background colors specifically; this mirrors the hardcoded black
/// already used below for cells with no data at all.
fn bg_to_rgb(c: vt100::Color) -> image::Rgb<u8> {
    match c {
        vt100::Color::Default => image::Rgb([0, 0, 0]),
        other => color::to_rgb(other),
    }
}

/// The per-cell state that actually reaches the rasterizer: the resolved
/// glyph, its already-`to_rgb`'d foreground and background, and the three
/// attributes `render_screen` acts on.
///
/// **Invariant: quiescence must compare exactly what `render_screen`
/// reads.** `pty::Session`'s quiescence polling compares these values and
/// `render_screen` rasterizes them, from this one extraction path — so a
/// redraw that would change a pixel cannot be invisible to the wait that
/// decides the redraw is finished. Historically the two were maintained
/// separately (quiescence on `vt100::Screen::contents()`, plain text only)
/// and silently diverged, which is exactly how a colour-only fade-in got
/// captured at its blackest instant (#139, #131). Anything added here must
/// therefore be something `render_screen` genuinely renders, and anything
/// `render_screen` starts rendering must be added here.
///
/// Deliberately *not* included: `vt100`'s cursor position and visibility,
/// title, and dim/italic/strikethrough — `render_screen` renders none of
/// them, so letting them count as "still changing" would make a blinking
/// cursor look like an unfinished draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ObservableCell {
    ch: char,
    fg: image::Rgb<u8>,
    bg: image::Rgb<u8>,
    bold: bool,
    underline: bool,
    inverse: bool,
}

/// Extracts one cell's observable state. `None` — a position the parser
/// has no cell for at all — is the blank default `render_screen` has
/// always drawn for it: a space in the default foreground on black.
fn observable_cell(cell: Option<&vt100::Cell>) -> ObservableCell {
    match cell {
        Some(c) => ObservableCell {
            // Known caveat: this takes only the cell's first char, so a
            // combining mark stacked onto a base character (rare in
            // TTUI's own glyph set, but not impossible in arbitrary
            // terminal output) is silently dropped rather than composed,
            // and a double-width glyph's trailing continuation cell
            // (which `vt100` represents as an empty-string cell) renders
            // as blank rather than widened. Not a known issue in practice
            // for TTUI's current widgets, which draw single-width glyphs.
            ch: c.contents().chars().next().unwrap_or(' '),
            fg: color::to_rgb(c.fgcolor()),
            bg: bg_to_rgb(c.bgcolor()),
            bold: c.bold(),
            underline: c.underline(),
            inverse: c.inverse(),
        },
        None => ObservableCell {
            ch: ' ',
            fg: color::to_rgb(vt100::Color::Default),
            bg: image::Rgb([0u8, 0, 0]),
            bold: false,
            underline: false,
            inverse: false,
        },
    }
}

/// The whole screen's observable state, row-major (index
/// `row * cols + col`), as both `render_screen` and `pty`'s quiescence
/// polling see it.
///
/// **Invariant: quiescence must compare exactly what `render_screen`
/// reads** — see `ObservableCell`. `render_screen` consumes this exact
/// function's output rather than re-deriving cell state of its own, so
/// the signal and the artifact cannot silently diverge.
pub(crate) fn observable_screen(screen: &vt100::Screen) -> Vec<ObservableCell> {
    let (rows, cols) = screen.size();
    let mut cells = Vec::with_capacity(rows as usize * cols as usize);
    for row in 0..rows {
        for col in 0..cols {
            cells.push(observable_cell(screen.cell(row, col)));
        }
    }
    cells
}

/// Rasterizes a parsed terminal screen to a 2x-upscaled RGBA image,
/// one 16x16 block per cell.
///
/// Reads the screen only through `observable_screen`, which is also what
/// quiescence compares — see `ObservableCell` for why that shared path is
/// load-bearing rather than incidental.
pub fn render_screen(screen: &vt100::Screen) -> Result<RgbaImage, RenderError> {
    let (rows, cols) = screen.size();
    let mut img = RgbaImage::new(cols as u32 * CELL_PX, rows as u32 * CELL_PX);

    for (index, cell) in observable_screen(screen).into_iter().enumerate() {
        // Row-major, as documented on `observable_screen`. `cols` cannot
        // be zero here: a zero-column screen yields no cells at all, so
        // this body never runs.
        let row = (index / cols as usize) as u16;
        let col = (index % cols as usize) as u16;

        let mut fg = cell.fg;
        let mut bg = cell.bg;
        if cell.bold {
            fg = color::brighten(fg);
        }
        if cell.inverse {
            let (nf, nb) = color::swap(fg, bg);
            fg = nf;
            bg = nb;
        }

        let bitmap = glyph::glyph_for(cell.ch).map_err(|e| RenderError::Glyph(e, row, col))?;

        let ox = col as u32 * CELL_PX;
        let oy = row as u32 * CELL_PX;
        for gy in 0..GLYPH_PX {
            let row_bits = bitmap[gy as usize];
            for gx in 0..GLYPH_PX {
                let set = (row_bits >> gx) & 1 == 1;
                let px = if set { fg } else { bg };
                for sy in 0..SCALE {
                    for sx in 0..SCALE {
                        img.put_pixel(
                            ox + gx * SCALE + sx,
                            oy + gy * SCALE + sy,
                            Rgba([px.0[0], px.0[1], px.0[2], 255]),
                        );
                    }
                }
            }
        }

        if cell.underline {
            let fg_px = Rgba([fg.0[0], fg.0[1], fg.0[2], 255]);
            for x in 0..CELL_PX {
                img.put_pixel(ox + x, oy + CELL_PX - 1, fg_px);
            }
        }
    }

    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL_PX: u32 = 16; // 8x8 glyph, 2x upscaled

    fn parse(bytes: &[u8]) -> vt100::Parser {
        let mut p = vt100::Parser::new(1, 4, 0);
        p.process(bytes);
        p
    }

    #[test]
    #[allow(clippy::identity_op)] // `1 * CELL_PX` spells out "1 row" for symmetry with `4 * CELL_PX` above it
    fn image_dimensions_match_screen_size_times_cell_px() {
        let parser = parse(b"abcd");
        let img = render_screen(parser.screen()).unwrap();
        assert_eq!(img.width(), 4 * CELL_PX);
        assert_eq!(img.height(), 1 * CELL_PX);
    }

    #[test]
    fn plain_text_uses_default_fg_over_default_bg() {
        let parser = parse(b"a");
        let img = render_screen(parser.screen()).unwrap();
        // A glyph's background rectangle should show through wherever
        // the 8x8 'a' bitmap has no set pixel — check a corner pixel
        // known to be background for every font8x8 letterform.
        let bg_pixel = img.get_pixel(0, 0);
        assert_eq!(*bg_pixel, image::Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn bold_text_brightens_the_foreground() {
        let parser = parse(b"\x1b[1ma\x1b[0m");
        let img = render_screen(parser.screen()).unwrap();
        // Find a foreground-colored pixel (non-background) and confirm
        // it's brighter than the plain-text case's foreground would be.
        let has_bright_pixel = img.pixels().any(|p| p.0[0] > 229);
        assert!(has_bright_pixel, "expected a brightened foreground pixel");
    }

    #[test]
    fn reverse_video_swaps_fg_and_bg_across_the_whole_cell() {
        let parser = parse(b"\x1b[7ma\x1b[0m");
        let img = render_screen(parser.screen()).unwrap();
        // With fg/bg swapped, the corner background pixel becomes the
        // (light) default foreground color instead of black.
        let corner = img.get_pixel(0, 0);
        assert_eq!(*corner, image::Rgba([229, 229, 229, 255]));
    }

    #[test]
    fn underline_draws_a_line_on_the_bottom_row_of_the_cell() {
        let parser = parse(b"\x1b[4m \x1b[0m"); // underlined space
        let img = render_screen(parser.screen()).unwrap();
        let bottom_row_pixel = img.get_pixel(0, CELL_PX - 1);
        assert_eq!(*bottom_row_pixel, image::Rgba([229, 229, 229, 255]));
    }

    /// The defect this whole Arc exists to close, stated at the level of
    /// the extraction function: `Screen::contents()` — the old quiescence
    /// signal — reports these two screens as identical, because the only
    /// difference between them is background colour. `observable_screen`
    /// is what quiescence compares instead precisely so that a redraw
    /// `render_screen` would rasterize differently can never be mistaken
    /// for "nothing changed". The `assert_eq!` on `contents()` is not
    /// incidental: it pins the precondition, so this test still proves
    /// something if `vt100` ever starts reporting attributes there.
    #[test]
    fn observable_screen_distinguishes_identical_text_over_different_backgrounds() {
        let plain = parse(b"abcd");
        let on_red = parse(b"\x1b[41mabcd\x1b[0m");

        assert_eq!(
            plain.screen().contents(),
            on_red.screen().contents(),
            "precondition: the old plain-text signal cannot tell these apart"
        );
        assert_ne!(
            observable_screen(plain.screen()),
            observable_screen(on_red.screen()),
            "observable_screen must see the background change render_screen renders"
        );
    }

    #[test]
    fn unmapped_glyph_is_a_hard_error_naming_the_codepoint_and_position() {
        let mut p = vt100::Parser::new(1, 1, 0);
        p.process("\u{2726}".as_bytes());
        let err = render_screen(p.screen()).unwrap_err();
        assert_eq!(
            err,
            RenderError::Glyph(glyph::GlyphError::Unmapped('\u{2726}'), 0, 0)
        );
    }
}
