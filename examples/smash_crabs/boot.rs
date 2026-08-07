use super::*;

fn render_centered_art(buf: &mut Buffer, area: Rect, art: &[&str], fg: Color) {
    let art_height = art.len() as u16;
    let art_width = art
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as u16;
    let y0 = area.height.saturating_sub(art_height) / 3;
    let x0 = area.width.saturating_sub(art_width) / 2;
    for (row, line) in art.iter().enumerate() {
        let y = y0 + row as u16;
        if y >= area.height {
            break;
        }
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = x0 + col as u16;
            if x < area.width {
                buf.set(
                    area.x + x,
                    area.y + y,
                    Cell {
                        symbol: ch,
                        fg,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

fn render_boot_title(buf: &mut Buffer, area: Rect, sub: f32, fg: Color) {
    let chars: Vec<char> = BOOT_TITLE.chars().collect();
    let half = chars.len() / 2;
    let total_width = chars.len() as u16;
    let start_x = area.width.saturating_sub(total_width) / 2;
    let y = area.height * 2 / 3;
    if y >= area.height {
        return;
    }
    for (i, &ch) in chars.iter().enumerate() {
        if ch == ' ' {
            continue;
        }
        let final_x = (start_x + i as u16) as f32;
        let x = if i < half {
            let from_x = -((half - i) as f32) - 2.0;
            easing::ease_out(from_x, final_x, sub)
        } else {
            let from_x = area.width as f32 + (i - half) as f32 + 2.0;
            easing::ease_out(from_x, final_x, sub)
        };
        let x = x.round();
        if x >= 0.0 && (x as u16) < area.width {
            buf.set(
                area.x + x as u16,
                area.y + y,
                Cell {
                    symbol: ch,
                    fg,
                    bg: Color::Reset,
                    ..Default::default()
                },
            );
        }
    }
}

impl SmashCrabs {
    pub(crate) fn render_boot(&self, area: Rect, progress: f32, buf: &mut LayerStack) {
        let t1 = BOOT_FLASH_MS as f32 / BOOT_TOTAL_MS as f32;
        let t2 = (BOOT_FLASH_MS + BOOT_CLAW_MS) as f32 / BOOT_TOTAL_MS as f32;
        let t3 = (BOOT_FLASH_MS + BOOT_CLAW_MS + BOOT_TITLE_MS) as f32 / BOOT_TOTAL_MS as f32;
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };

        if progress < t1 {
            let sub = progress / t1;
            let mut white = Buffer::new(area.width, area.height);
            let cell = Cell {
                symbol: ' ',
                fg: Color::Reset,
                bg: Color::White,
                ..Default::default()
            };
            for y in 0..area.height {
                for x in 0..area.width {
                    white.set(x, y, cell.clone());
                }
            }
            let dimmed = camera::dim(&white, sub);
            blit(&dimmed, area, buf);
            return;
        }

        if progress < t2 {
            let sub = (progress - t1) / (t2 - t1);
            let art: &[&str] = if sub < 0.5 { &CLAW_OPEN } else { &CLAW_CLOSED };
            render_centered_art(buf, area, art, self.theme.tertiary);
            return;
        }

        if progress < t3 {
            let sub = (progress - t2) / (t3 - t2);
            render_centered_art(buf, area, &CLAW_CLOSED, self.theme.tertiary);
            render_boot_title(buf, area, sub, self.theme.accent);
            return;
        }

        let sub = ((progress - t3) / (1.0 - t3)).clamp(0.0, 1.0);
        let hub_content = self.render_destination_preview(Screen::Hub, area);
        let mut logo = Buffer::new(area.width, area.height);
        render_centered_art(&mut logo, local, &CLAW_CLOSED, self.theme.tertiary);
        render_boot_title(&mut logo, local, 1.0, self.theme.accent);

        let flare_x = -3.0 + sub * (area.width as f32 + 6.0);
        for y in 0..area.height {
            for x in 0..area.width {
                let fx = x as f32;
                let cell = if (fx - flare_x).abs() <= 1.5 {
                    let h = (x as u64).wrapping_mul(374_761_393)
                        ^ self.tick_count.wrapping_mul(2_246_822_519);
                    let fg = if h.is_multiple_of(2) {
                        self.theme.accent
                    } else {
                        self.theme.tertiary
                    };
                    Cell {
                        symbol: '|',
                        fg,
                        bg: Color::Reset,
                        ..Default::default()
                    }
                } else if fx < flare_x {
                    hub_content.get(x, y).clone()
                } else {
                    logo.get(x, y).clone()
                };
                buf.set(area.x + x, area.y + y, cell);
            }
        }
    }
}
