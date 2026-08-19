// examples/launcher/portal.rs — one portal tile: a themed Block frame
// plus a centered glyph, app name, and tagline. The focused portal
// glows (thick border ring) and pulses.
use crossterm::style::Color;
use ttui::buffer::{Buffer, CellStyle, Intensity};
use ttui::layout::Rect;
use ttui::theme::{BorderSet, Theme};
use ttui::widgets::block::Block;

use crate::{text_center, VOID};
use ttui::easing::scale_color;

/// Draws the portal for one app into `scene` within `slot`.
pub(crate) fn draw(
    scene: &mut Buffer,
    slot: Rect,
    name: &str,
    tagline: &str,
    accent: Color,
    focused: bool,
    phase: f32,
) {
    let pulse = ((phase * 3.0).sin() + 1.0) / 2.0;
    let border = if focused {
        accent
    } else {
        scale_color(accent, 0.4)
    };
    let theme = Theme {
        background: VOID,
        primary: border,
        secondary: accent,
        tertiary: accent,
        accent,
        primary_end: if focused {
            Some(scale_color(accent, 0.3 + 0.7 * pulse))
        } else {
            None
        },
        border: BorderSet {
            horizontal: '─',
            vertical: '│',
            top_left: if focused { '◆' } else { '·' },
            top_right: if focused { '◆' } else { '·' },
            bottom_left: if focused { '◆' } else { '·' },
            bottom_right: if focused { '◆' } else { '·' },
        },
        border_style: CellStyle {
            intensity: if focused && pulse > 0.5 {
                Intensity::Bold
            } else {
                Intensity::Normal
            },
            ..Default::default()
        },
        border_thick: focused,
    };
    let inner = Block::new().theme(&theme).render(slot, scene);
    if inner.height < 3 {
        return;
    }

    let mid = inner.y + inner.height / 2;
    let glyph = if focused { '◉' } else { '○' };
    let glyph_color = if focused {
        scale_color(accent, 0.6 + 0.4 * pulse)
    } else {
        scale_color(accent, 0.6)
    };
    text_center(
        scene,
        inner,
        mid.saturating_sub(1),
        &glyph.to_string(),
        glyph_color,
        focused,
    );
    text_center(scene, inner, mid, name, accent, focused);
    text_center(
        scene,
        inner,
        mid + 1,
        tagline,
        scale_color(accent, 0.6),
        false,
    );

    if focused {
        let prompt_row = inner.y + inner.height.saturating_sub(1);
        text_center(scene, inner, prompt_row, "◄ ENTER ►", accent, true);
    }
}
