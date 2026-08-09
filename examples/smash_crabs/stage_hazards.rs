use super::*;

impl SmashCrabs {
    pub(crate) fn sh_cpu(&self) -> f32 {
        50.0 + 15.0 * (self.tick_count as f32 * 0.03).sin()
    }

    pub(crate) fn render_stage_hazards(&self, area: Rect, buf: &mut LayerStack) {
        let inner = SmashBorder::new().render(area, &self.theme, buf);
        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fixed(1), Constraint::Fixed(1)],
        )
        .split(inner);

        Text::new("CPU").render(
            Rect {
                x: rows[0].x,
                y: rows[0].y,
                width: 4.min(rows[0].width),
                height: 1,
            },
            buf,
        );
        DamageMeter::new(self.sh_cpu().round() as u16).render(
            Rect {
                x: rows[0].x + 4,
                y: rows[0].y,
                width: rows[0].width.saturating_sub(4),
                height: 1,
            },
            buf,
        );
        Text::new("RAM").render(
            Rect {
                x: rows[1].x,
                y: rows[1].y,
                width: 4.min(rows[1].width),
                height: 1,
            },
            buf,
        );
        DamageMeter::new(self.sh_ram.round() as u16).render(
            Rect {
                x: rows[1].x + 4,
                y: rows[1].y,
                width: rows[1].width.saturating_sub(4),
                height: 1,
            },
            buf,
        );

        if self.sh_ram >= RAM_THRESHOLD {
            let flashing_on = (self.tick_count / BOBOMB_FLASH_TICKS).is_multiple_of(2);
            let color = if flashing_on {
                Color::Red
            } else {
                self.theme.background
            };
            let art_width = BOBOMB_ART[0].chars().count() as u16;
            let art_x = area.x + area.width.saturating_sub(art_width + 1);
            for (row, line) in BOBOMB_ART.iter().enumerate() {
                let y = area.y + 1 + row as u16;
                if y >= area.y + area.height {
                    break;
                }
                for (col, ch) in line.chars().enumerate() {
                    let x = art_x + col as u16;
                    if x < area.x + area.width {
                        buf.set(
                            x,
                            y,
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
            }
        }

        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new("Space stress RAM * Esc back * q quit").render(hint_row, buf);
    }
}
