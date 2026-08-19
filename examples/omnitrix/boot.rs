use super::*;

impl Omnitrix {
    pub(crate) fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        for y in 0..area.height {
            for x in 0..area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ' ',
                        fg: Color::Reset,
                        bg: Color::Black,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
            }
        }

        match BOOT.at(progress) {
            (0, t) => {
                // The hourglass dims away as the phase runs, so the phase's
                // own `t` has to be inverted here — the fade-out belongs to
                // this app, not to `Phases`, which always counts upward.
                let factor = 1.0 - t;
                let mut scratch = Buffer::new(5, 5);
                let theme = self.theme();
                for (row, line) in HOURGLASS.iter().enumerate() {
                    for (col, ch) in line.chars().enumerate() {
                        scratch.set(
                            col as u16,
                            row as u16,
                            Cell {
                                symbol: ch,
                                fg: theme.primary,
                                bg: Color::Reset,
                                alpha: 1.0,
                                ..Default::default()
                            },
                        );
                    }
                }
                let dimmed = camera::dim(&scratch, factor);
                let x0 = area.x + (area.width.saturating_sub(5)) / 2;
                let y0 = area.y + (area.height.saturating_sub(5)) / 2;
                dimmed.blit(buf, x0, y0);
            }

            (1, _) => {
                for y in 0..area.height {
                    for x in 0..area.width {
                        buf.set(
                            area.x + x,
                            area.y + y,
                            Cell {
                                symbol: ' ',
                                fg: Color::Reset,
                                bg: Color::Rgb {
                                    r: 0,
                                    g: 255,
                                    b: 65,
                                },
                                alpha: 1.0,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            (_, trace_progress) => {
                let scale = easing::ease_out(0.2, 1.0, trace_progress);
                let w = (((area.width as f32) * scale) as u16)
                    .max(2)
                    .min(area.width);
                let h = (((area.height as f32) * scale) as u16)
                    .max(2)
                    .min(area.height);
                let x = area.x + (area.width.saturating_sub(w)) / 2;
                let y = area.y + (area.height.saturating_sub(h)) / 2;
                let theme = self.theme();
                Block::new().title("Omnitrix").theme(&theme).render(
                    Rect {
                        x,
                        y,
                        width: w,
                        height: h,
                    },
                    buf,
                );
            }
        }
    }
}
