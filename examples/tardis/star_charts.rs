use super::*;

impl Tardis {
    pub(crate) fn render_star_charts(&self, area: Rect, buf: &mut LayerStack) {
        if let Some(t) = &self.temporal_shift {
            if t.progress() < 0.3 {
                for y in 0..area.height {
                    for x in 0..area.width {
                        buf.set(
                            area.x + x,
                            area.y + y,
                            Cell {
                                symbol: ' ',
                                fg: Color::Reset,
                                bg: self.theme.accent,
                                ..Default::default()
                            },
                        );
                    }
                }
                return;
            }
        }

        for (index, name) in TIMELINE.iter().enumerate() {
            let diff = (index + TIMELINE.len() - self.present_index) % TIMELINE.len();
            let row = index as u16;
            if row >= area.height {
                continue;
            }
            if diff == 0 {
                let pulse = ((self.tick_count as f32 * 0.1).sin() + 1.0) / 2.0;
                Roundel::new(pulse, self.theme.primary).render(
                    Rect {
                        x: area.x,
                        y: area.y + row,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
                let name_area = Rect {
                    x: area.x + 2,
                    y: area.y + row,
                    width: area.width.saturating_sub(2),
                    height: 1,
                };
                Text::new(name).render(name_area, buf);
            } else if diff == 3 || diff == 4 {
                let line = format!("◆ {name}");
                for (i, ch) in line.chars().take(area.width as usize).enumerate() {
                    buf.set(
                        area.x + i as u16,
                        area.y + row,
                        Cell {
                            symbol: ch,
                            fg: self.theme.accent,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            } else {
                for col in 0..12u16.min(area.width) {
                    let h = (col as u64).wrapping_mul(374_761_393)
                        ^ (row as u64).wrapping_mul(668_265_263)
                        ^ self.tick_count.wrapping_mul(2_246_822_519);
                    let glyph = CLOUD_GLYPHS[(h % 4) as usize];
                    buf.set(
                        area.x + col,
                        area.y + row,
                        Cell {
                            symbol: glyph,
                            fg: self.theme.secondary,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
        }

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Enter shift * Esc back * q quit").render(hint_row, buf);
    }
}
