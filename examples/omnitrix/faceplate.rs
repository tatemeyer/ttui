use super::*;

impl Omnitrix {
    pub(crate) fn render_faceplate_content(&self, local: Rect, buf: &mut Buffer) {
        let dial_area = Rect {
            x: local.x,
            y: local.y,
            width: local.width,
            height: local.height.saturating_sub(1),
        };
        let hint_row = Rect {
            x: local.x,
            y: local.y + local.height.saturating_sub(1),
            width: local.width,
            height: local.height.saturating_sub(1).min(1),
        };
        let names: Vec<String> = SAMPLES.iter().map(|s| s.to_string()).collect();
        Dial::new(&names, self.selected).render(dial_area, buf);
        Text::new("Tab/Shift+Tab cycle * Enter launch * q quit").render(hint_row, buf);
    }
}
