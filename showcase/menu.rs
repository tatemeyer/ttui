//! The tile menu: one row of 5 tiles, one per vignette. Arrow keys
//! move the highlight (mascot reacts on change, wired in showcase.rs);
//! Enter or a direct click on a tile launches it. Hover alone never
//! launches.

use super::VignetteId;
use ttui::buffer::LayerStack;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::theme::Theme;
use ttui::widgets::{block::Block, text::Text};

pub(crate) const TILES: [(VignetteId, &str, &str); 5] = [
    (VignetteId::AssemblyLine, "Assembly Line", "click"),
    (VignetteId::OverloadVent, "Overload Vent", "watch"),
    (
        VignetteId::DiagnosticScan,
        "Diagnostic Scan",
        "space to whack",
    ),
    (VignetteId::OverrideSequence, "Override Sequence", "chord"),
    (VignetteId::Telemetry, "Telemetry", "watch"),
];

const ZERO_RECT: Rect = Rect {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};

/// Renders the 5-tile row, returning each tile's outer `Rect` (border
/// included) for the caller to hit-test clicks against.
pub(crate) fn render_menu(
    area: Rect,
    theme: &Theme,
    highlighted: usize,
    buf: &mut LayerStack,
) -> [Rect; 5] {
    let tiles_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height.saturating_sub(1),
    };
    let cols = Layout::new(
        Direction::Horizontal,
        vec![Constraint::Fill(1); TILES.len()],
    )
    .split(tiles_area);
    let mut areas = [ZERO_RECT; 5];
    for (i, col) in cols.iter().enumerate() {
        let (_, title, hint) = TILES[i];
        let inner = Block::new().title(title).theme(theme).render(*col, buf);
        areas[i] = *col;
        let label = if i == highlighted {
            format!("> {hint}")
        } else {
            hint.to_string()
        };
        Text::new(&label).render(inner, buf);
    }
    let hint_row = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: area.height.saturating_sub(1).min(1),
    };
    Text::new("Left/Right move * Enter launch * click tile * q quit").render(hint_row, buf);
    areas
}
