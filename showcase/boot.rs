//! Startup materialization: a centered logo fades in from the theme's
//! background to its primary color over the boot `Transition`'s
//! progress — the same background-to-color fade shape `falcon`'s own
//! boot sequence uses, at showcase's own (shorter) duration.

use crossterm::style::Color;
use ttui::buffer::{Cell, LayerStack};
use ttui::easing;
use ttui::layout::Rect;
use ttui::theme::Theme;

const LOGO: &str = "GRIPPER SHOWCASE";

pub(crate) fn render_boot(area: Rect, theme: &Theme, progress: f32, buf: &mut LayerStack) {
    let color = easing::lerp_color(theme.background, theme.primary, progress.clamp(0.0, 1.0));
    let cx = area.x + area.width.saturating_sub(LOGO.chars().count() as u16) / 2;
    let cy = area.y + area.height / 2;
    for (i, ch) in LOGO.chars().enumerate() {
        let x = cx + i as u16;
        if x >= area.x + area.width {
            break;
        }
        buf.set(
            x,
            cy,
            Cell {
                symbol: ch,
                fg: color,
                bg: Color::Reset,
                alpha: 1.0,
                ..Default::default()
            },
        );
    }
}
