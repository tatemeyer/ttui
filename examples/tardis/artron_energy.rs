use super::*;

impl Tardis {
    pub(crate) fn render_artron_energy(&self, area: Rect, buf: &mut LayerStack) {
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        Text::new("Artron Energy").render(name_row, buf);

        for i in 0..3u16 {
            let seg_intensity = ((self.energy - i as f32 * 33.0) / 33.0).clamp(0.0, 1.0);
            let rx = area.x + 4 + i * 4;
            let ry = area.y + 2;
            Roundel::new(seg_intensity, self.theme.tertiary).render(
                Rect {
                    x: rx,
                    y: ry,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }

        let toggle_row = Rect {
            x: area.x,
            y: area.y + 4,
            width: area.width.min(10),
            height: 1,
        };
        AnalogToggle::new(self.vent_flash.is_some()).render(toggle_row, buf);

        let rotor_area = Rect {
            x: area.x,
            y: area.y + 6,
            width: area.width,
            height: area.height.saturating_sub(8),
        };
        TimeRotor::new(self.time_rotor_speed()).render(rotor_area, self.tick_count, buf);

        if self.glitch.is_active() {
            self.glitch.render(area, Color::Red, self.tick_count, buf);
        }

        self.particles.render(buf);

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Space channel * v vent * Esc back * q quit").render(hint_row, buf);
    }
}
