use super::*;

impl Tardis {
    pub(crate) fn render_psychic_paper(&self, area: Rect, buf: &mut LayerStack) {
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: PAPER_COLOR,
                        ..Default::default()
                    },
                );
            }
        }

        let start = self.psychic_log.len().saturating_sub(5);
        let last_index = self.psychic_log.len().saturating_sub(1);
        for (i, (speaker, text)) in self.psychic_log[start..].iter().enumerate() {
            let absolute_index = start + i;
            let prefix = match speaker {
                RelaySpeaker::User => "You: ",
                RelaySpeaker::Agent => "Relay: ",
            };
            let is_latest_agent = *speaker == RelaySpeaker::Agent
                && !self.psychic_log.is_empty()
                && absolute_index == last_index;
            let fg = if is_latest_agent {
                match &self.psychic_reveal {
                    Some(t) => lerp_color(PAPER_COLOR, INK_COLOR, t.progress()),
                    None => INK_COLOR,
                }
            } else {
                INK_COLOR
            };
            render_ink_row(buf, area, i as u16, &format!("{prefix}{text}"), fg);

            if is_latest_agent && self.glitch.is_active() && (i as u16) < area.height {
                let glitch_row = Rect {
                    x: area.x,
                    y: area.y + i as u16,
                    width: area.width,
                    height: 1,
                };
                self.glitch
                    .render(glitch_row, Color::Red, self.tick_count, buf);
            }
        }

        render_ink_row(
            buf,
            area,
            area.height.saturating_sub(2),
            PSYCHIC_PROMPTS[self.psychic_prompt_index],
            INK_COLOR,
        );
        render_ink_row(
            buf,
            area,
            area.height.saturating_sub(1),
            "Tab cycle * Enter send * Esc back * q quit",
            INK_COLOR,
        );
    }
}
