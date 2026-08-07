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
                        ..Default::default()
                    },
                );
            }
        }

        if progress < 0.4 {
            let factor = (1.0 - progress / 0.4).clamp(0.0, 1.0);
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
                            ..Default::default()
                        },
                    );
                }
            }
            let dimmed = camera::dim(&scratch, factor);
            let x0 = area.x + (area.width.saturating_sub(5)) / 2;
            let y0 = area.y + (area.height.saturating_sub(5)) / 2;
            blit(
                &dimmed,
                Rect {
                    x: x0,
                    y: y0,
                    width: 5,
                    height: 5,
                },
                buf,
            );
            return;
        }

        if progress < 0.55 {
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
                            ..Default::default()
                        },
                    );
                }
            }
            return;
        }

        let trace_progress = ((progress - 0.55) / 0.45).clamp(0.0, 1.0);
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
