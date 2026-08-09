use super::*;

impl Omnitrix {
    pub(crate) fn render_circuit(&self, area: Rect, value: f32, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let lit = ((value.min(100.0) / 100.0) * CIRCUIT_NODE_COUNT as f32) as u16;
        let theme = self.theme();
        let mut x: u16 = 0;
        for i in 0..CIRCUIT_NODE_COUNT {
            if x >= area.width {
                break;
            }
            let (symbol, color) = if i < lit {
                ('●', theme.primary)
            } else {
                ('○', theme.secondary)
            };
            buf.set(
                area.x + x,
                area.y,
                Cell {
                    symbol,
                    fg: color,
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
            x += 1;
            if i + 1 < CIRCUIT_NODE_COUNT && x < area.width {
                buf.set(
                    area.x + x,
                    area.y,
                    Cell {
                        symbol: '─',
                        fg: theme.secondary,
                        bg: Color::Reset,
                        alpha: 1.0,
                        ..Default::default()
                    },
                );
                x += 1;
            }
        }
    }

    pub(crate) fn render_upgrade_content(&self, local: Rect, buf: &mut Buffer) {
        let cpu_label = Rect {
            x: local.x,
            y: local.y,
            width: local.width,
            height: 1,
        };
        Text::new("CPU").render(cpu_label, buf);
        let cpu_row = Rect {
            x: local.x,
            y: local.y + 1,
            width: local.width,
            height: 1,
        };
        self.render_circuit(cpu_row, self.load, buf);

        let ram_value = (self.load * 0.6 + 10.0).min(100.0);
        let ram_label = Rect {
            x: local.x,
            y: local.y + 3,
            width: local.width,
            height: 1,
        };
        Text::new("RAM").render(ram_label, buf);
        let ram_row = Rect {
            x: local.x,
            y: local.y + 4,
            width: local.width,
            height: 1,
        };
        self.render_circuit(ram_row, ram_value, buf);

        let hint_row = Rect {
            x: local.x,
            y: local.y + local.height.saturating_sub(1),
            width: local.width,
            height: local.height.saturating_sub(1).min(1),
        };
        Text::new("Space overload * Esc back * q quit").render(hint_row, buf);
    }
}
