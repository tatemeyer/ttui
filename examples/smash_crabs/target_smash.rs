use super::*;

impl SmashCrabs {
    pub(crate) fn ts_visible(&self) -> Vec<usize> {
        (0..TS_TARGETS.len())
            .filter(|&i| !self.ts_smashed[i])
            .collect()
    }

    pub(crate) fn ts_smashing_is_impact(&self) -> bool {
        matches!(&self.ts_smashing, Some((_, TsPhase::Impact(_))))
    }

    pub(crate) fn paint_ts_ui(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let inner = SmashBorder::new().render(local, &self.theme, &mut buf);
        let visible = self.ts_visible();
        if visible.is_empty() {
            render_row(&mut buf, inner, "ALL TARGETS DOWN", self.theme.tertiary);
        } else {
            for (row, &real_index) in visible.iter().enumerate() {
                let y = inner.y + row as u16;
                if y >= inner.y + inner.height {
                    break;
                }
                let fg = match &self.ts_smashing {
                    Some((i, TsPhase::Fade(t))) if *i == real_index => {
                        easing::lerp_color(self.theme.tertiary, self.theme.background, t.progress())
                    }
                    _ if row == self.ts_selected => self.theme.accent,
                    _ => self.theme.tertiary,
                };
                let row_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                };
                render_row(&mut buf, row_area, TS_TARGETS[real_index], fg);
            }
        }
        let hint_row = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: inner.height.saturating_sub(1).min(1),
        };
        Text::new("Up/Down move * Enter smash * Esc back * q quit").render(hint_row, &mut buf);
        buf
    }

    pub(crate) fn paint_ts_effects(&self, area: Rect) -> Buffer {
        let mut buf = Buffer::new(area.width, area.height);
        if self.ts_smashing_is_impact() {
            let cx = area.width / 2;
            let cy = area.height / 2;
            for offset in [-4i32, 0, 4] {
                let x = cx as i32 + offset;
                if x >= 0 && (x as u16) < area.width && cy > 0 {
                    buf.set(
                        x as u16,
                        cy - 1,
                        Cell {
                            symbol: TS_IMPACT_GLYPH,
                            fg: self.theme.accent,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
            let ko_x = cx.saturating_sub(1);
            for (i, ch) in "KO".chars().enumerate() {
                let x = ko_x + i as u16;
                if x < area.width && cy + 1 < area.height {
                    buf.set(
                        x,
                        cy + 1,
                        Cell {
                            symbol: ch,
                            fg: self.theme.tertiary,
                            bg: self.theme.primary,
                            style: CellStyle {
                                bold: true,
                                ..Default::default()
                            },
                        },
                    );
                }
            }
        }
        buf
    }

    pub(crate) fn render_target_smash(&self, area: Rect, buf: &mut LayerStack) {
        let (dx, dy) = self.shake_offset();
        let layers: [(usize, Buffer); 3] = [
            (BACKGROUND, self.paint_background(area)),
            (UI, self.paint_ts_ui(area)),
            (EFFECTS, self.paint_ts_effects(area)),
        ];
        for (index, scratch) in layers {
            let final_buf = if dx != 0 || dy != 0 {
                effects::shake(&scratch, dx, dy)
            } else {
                scratch
            };
            blit(&final_buf, area, buf.layer_mut(index));
        }
    }
}
