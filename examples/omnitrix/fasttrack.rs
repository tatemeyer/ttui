use super::*;

impl Omnitrix {
    pub(crate) fn active_target_indices(&self) -> Vec<usize> {
        self.targets
            .iter()
            .enumerate()
            .filter(|(_, (_, done))| !done)
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn render_lock_on_ring(&self, area: Rect, progress: f32, buf: &mut Buffer) {
        let cx = area.x as f32 + area.width as f32 / 2.0;
        let cy = area.y as f32 + area.height as f32 / 2.0;
        let radius_x = 4.0;
        let radius_y = 2.0;
        let lit_count = (progress * RING_POINTS as f32) as usize;
        let theme = self.theme();
        for i in 0..RING_POINTS {
            let angle =
                i as f32 * std::f32::consts::TAU / RING_POINTS as f32 - std::f32::consts::FRAC_PI_2;
            let px = (cx + radius_x * angle.cos()).round();
            let py = (cy + radius_y * angle.sin()).round();
            if px >= area.x as f32
                && py >= area.y as f32
                && (px as u16) < area.x + area.width
                && (py as u16) < area.y + area.height
            {
                let (symbol, color) = if i < lit_count {
                    ('●', theme.primary)
                } else {
                    ('○', theme.secondary)
                };
                buf.set(
                    px as u16,
                    py as u16,
                    Cell {
                        symbol,
                        fg: color,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
        }
    }

    pub(crate) fn render_fasttrack_content(&self, local: Rect, buf: &mut Buffer) {
        let active = self.active_target_indices();
        let mut y: u16 = 0;

        render_row(buf, local, y, "Targets", Color::Reset, Color::Reset);
        y += 1;
        for (row, &idx) in active.iter().enumerate() {
            let is_selected = row == self.target_selected;
            let (fg, bg) = if is_selected {
                (Color::Black, Color::White)
            } else {
                (Color::Reset, Color::Reset)
            };
            let line = format!("○ {}", self.targets[idx].0);
            render_row(buf, local, y, &line, fg, bg);
            y += 1;
        }
        y += 1;

        if let Some((_, t)) = &self.lock_on {
            if y < local.height {
                let ring_area = Rect {
                    x: local.x,
                    y: local.y + y,
                    width: 9.min(local.width),
                    height: 5.min(local.height.saturating_sub(y)),
                };
                self.render_lock_on_ring(ring_area, t.progress(), buf);
            }
        }
        y += 6;

        render_row(buf, local, y, "Completed", Color::Reset, Color::Reset);
        y += 1;
        let completed: Vec<&(String, bool)> =
            self.targets.iter().filter(|(_, done)| *done).collect();
        let completed_len = completed.len();
        for (row, (name, _)) in completed.iter().enumerate() {
            let flashing = self.complete_flash.is_some() && row + 1 == completed_len;
            let bg = if flashing {
                self.theme().accent
            } else {
                Color::Reset
            };
            let line = format!("◉ {name}");
            render_row(buf, local, y, &line, self.theme().secondary, bg);
            y += 1;
        }
        y += 1;

        let percent = (completed_len as u32 * 100 / 3) as u16;
        if y < local.height {
            let bar_area = Rect {
                x: local.x,
                y: local.y + y,
                width: local.width,
                height: 1,
            };
            EnergyCore::new(percent, self.theme().primary).render(bar_area, buf);
        }

        let hint_row = Rect {
            x: local.x,
            y: local.y + local.height.saturating_sub(1),
            width: local.width,
            height: local.height.saturating_sub(1).min(1),
        };
        Text::new("Tab cycle * Enter lock-on * Esc back * q quit").render(hint_row, buf);
    }
}
