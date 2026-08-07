use super::*;

impl Omnitrix {
    pub(crate) fn render_brainstorm_content(&self, local: Rect, buf: &mut Buffer) {
        let log_area = Rect {
            x: local.x,
            y: local.y,
            width: local.width,
            height: local.height.saturating_sub(2),
        };
        let start = self.chat_log.len().saturating_sub(5);
        for (i, (speaker, text)) in self.chat_log[start..].iter().enumerate() {
            let prefix = match speaker {
                ChatSpeaker::User => "You: ",
                ChatSpeaker::Agent => "Agent: ",
            };
            render_row(
                buf,
                log_area,
                i as u16,
                &format!("{prefix}{text}"),
                Color::Reset,
                Color::Reset,
            );
        }

        let input_row = Rect {
            x: local.x,
            y: local.y + local.height.saturating_sub(2),
            width: local.width,
            height: 1,
        };
        let prompt = CANNED_PROMPTS[self.prompt_index];
        let reveal_len =
            ((prompt.chars().count() as f32) * self.preview_reveal.progress()) as usize;
        let preview = &prompt[..reveal_len.min(prompt.len())];
        let theme = self.theme();
        DNAConsole::new(preview, theme.primary, theme.secondary).render(input_row, buf);

        let hint_row = Rect {
            x: local.x,
            y: local.y + local.height.saturating_sub(1),
            width: local.width,
            height: local.height.saturating_sub(1).min(1),
        };
        Text::new("Tab cycle * Enter send * Esc back * q quit").render(hint_row, buf);
    }
}
