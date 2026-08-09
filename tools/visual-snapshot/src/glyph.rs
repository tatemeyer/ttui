//! Looks up 8x8 bitmaps for characters TTUI actually draws, across the
//! specific `font8x8` tables that cover them. Deliberately does not fall
//! back to silently blank glyphs: an unmapped codepoint is a hard error.

use font8x8::{UnicodeFonts, BASIC_FONTS, BLOCK_FONTS, BOX_FONTS, LATIN_FONTS, MISC_FONTS};

/// A codepoint that none of the checked `font8x8` tables cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphError {
    /// The unmapped character.
    Unmapped(char),
}

/// Looks up `ch`'s 8x8 bitmap across every font8x8 table TTUI's actual
/// glyph set draws from. Returns a hard error naming the codepoint if
/// none of them cover it — see `GlyphError::Unmapped`.
pub fn glyph_for(ch: char) -> Result<[u8; 8], GlyphError> {
    BASIC_FONTS
        .get(ch)
        .or_else(|| LATIN_FONTS.get(ch))
        .or_else(|| BLOCK_FONTS.get(ch))
        .or_else(|| BOX_FONTS.get(ch))
        .or_else(|| MISC_FONTS.get(ch))
        .ok_or(GlyphError::Unmapped(ch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_letter_resolves_to_a_nonblank_bitmap() {
        let bitmap = glyph_for('A').unwrap();
        assert!(bitmap.iter().any(|row| *row != 0), "expected 'A' to draw at least one pixel");
    }

    #[test]
    fn space_and_a_resolve_to_different_bitmaps() {
        assert_ne!(glyph_for(' ').unwrap(), glyph_for('A').unwrap());
    }

    #[test]
    fn block_element_glyphs_resolve() {
        // The block-elements TTUI widgets actually emit (src/glitch.rs, src/canvas.rs).
        for ch in ['░', '▒', '▓', '█', '▀', '▄', '▌'] {
            glyph_for(ch).unwrap_or_else(|e| panic!("expected {ch:?} to be mapped, got {e:?}"));
        }
    }

    #[test]
    fn ascii_border_glyphs_resolve() {
        for ch in ['-', '|', '+'] {
            glyph_for(ch).unwrap();
        }
    }

    #[test]
    fn dingbat_star_is_unmapped() {
        // Confirmed during spec review: font8x8's MISC_FONTS does not
        // reach the Dingbats block. EnergyCore's charged-state glyph
        // is expected to hit this path in real use.
        let err = glyph_for('\u{2726}').unwrap_err();
        assert_eq!(err, GlyphError::Unmapped('\u{2726}'));
    }
}
