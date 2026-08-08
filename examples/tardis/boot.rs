use super::*;

impl Tardis {
    pub(crate) fn render_police_box(
        &self,
        area: Rect,
        lines: &[&str; 5],
        dx: i16,
        dy: i16,
        buf: &mut LayerStack,
    ) {
        let box_width: i32 = 8;
        let box_height: i32 = 5;
        let x0 = area.x as i32 + (area.width as i32 - box_width) / 2 + dx as i32;
        let y0 = area.y as i32 + (area.height as i32 - box_height) / 2 + dy as i32;
        for (row, line) in lines.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let px = x0 + col as i32;
                let py = y0 + row as i32;
                if px >= area.x as i32
                    && py >= area.y as i32
                    && (px as u16) < area.x + area.width
                    && (py as u16) < area.y + area.height
                {
                    buf.set(
                        px as u16,
                        py as u16,
                        Cell {
                            symbol: ch,
                            fg: self.theme.tertiary,
                            bg: Color::Reset,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

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

        if progress < 0.15 {
            self.render_police_box(area, &POLICE_BOX_CLOSED, 0, 0, buf);
            return;
        }
        if progress < 0.35 {
            let magnitude: i16 = 2;
            let dx = if self.tick_count.is_multiple_of(2) {
                magnitude
            } else {
                -magnitude
            };
            let dy = if (self.tick_count / 2).is_multiple_of(2) {
                magnitude
            } else {
                -magnitude
            };
            self.render_police_box(area, &POLICE_BOX_CLOSED, dx, dy, buf);
            return;
        }
        if progress < 0.5 {
            self.render_police_box(area, &POLICE_BOX_OPEN, 0, 0, buf);
            return;
        }
        if progress < 0.65 {
            for y in 0..area.height {
                for x in 0..area.width {
                    buf.set(
                        area.x + x,
                        area.y + y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: Color::Rgb {
                                r: 255,
                                g: 255,
                                b: 255,
                            },
                            ..Default::default()
                        },
                    );
                }
            }
            return;
        }

        let push_progress = ((progress - 0.65) / 0.35).clamp(0.0, 1.0);
        let zoom = easing::ease_out(1.0, 2.2, push_progress);
        let local = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: area.height,
        };
        let mut hub_stack = LayerStack::new(area.width, area.height);
        self.render_hub(local, &mut hub_stack);
        let cam = Camera::new(
            area.width as f32 / 2.0 * (1.0 - 1.0 / zoom),
            area.height as f32 / 2.0 * (1.0 - 1.0 / zoom),
            zoom,
        );
        let zoomed = camera::viewport(&hub_stack, &cam, area.width, area.height);
        blit(&zoomed, area, buf);
    }
}
