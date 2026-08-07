use super::*;

impl SmashCrabs {
    pub(crate) fn render_hub(&self, area: Rect, buf: &mut LayerStack) {
        let inner = SmashBorder::new().render(area, &self.theme, buf);
        let panels = Self::hub_panels(inner);
        for (i, panel) in panels.iter().enumerate() {
            let name_row = Rect {
                x: panel.x,
                y: panel.y,
                width: panel.width,
                height: panel.height.min(1),
            };
            Text::new(FIGHTERS[i]).render(name_row, buf);
        }
        let (cx, cy) = self.cursor_position(inner);
        ScuttleCursor::new(CURSOR_SYMBOL).render(
            cx,
            cy,
            self.cursor_tween.is_some(),
            self.tick_count,
            buf,
        );
        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new("Left/Right move * Enter select * q quit").render(hint_row, buf);
    }
}
