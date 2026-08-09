use super::*;

impl Tardis {
    pub(crate) fn render_face_content(&self, face: usize, area: Rect, buf: &mut Buffer) {
        let name_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.min(1),
        };
        Text::new(FACE_NAMES[face]).render(name_row, buf);
        if screen_for_face(face).is_none() {
            for i in 0..3u16 {
                let rx = area.x + (area.width / 4) * (i + 1);
                let ry = area.y + area.height / 2;
                let pulse = ((self.tick_count as f32 * 0.05 + i as f32).sin() + 1.0) / 2.0;
                Roundel::new(pulse, self.theme.tertiary, 1).render(
                    Rect {
                        x: rx.saturating_sub(1),
                        y: ry.saturating_sub(1),
                        width: 3,
                        height: 3,
                    },
                    buf,
                );
            }
        }
    }

    pub(crate) fn render_hub(&self, area: Rect, buf: &mut LayerStack) {
        let vw = area.width;
        let vh = area.height;
        let mut virtual_buf = Buffer::new(vw * FACE_COUNT as u16, vh);
        for face in 0..FACE_COUNT {
            let face_area = Rect {
                x: face as u16 * vw,
                y: 0,
                width: vw,
                height: vh,
            };
            self.render_face_content(face, face_area, &mut virtual_buf);
            let factor = DIM_FACTORS[hex_distance(face, self.selected_face)];
            if factor > 0.0 {
                let face_camera = Camera::new(face_area.x as f32, face_area.y as f32, 1.0);
                let cropped = camera::viewport(&virtual_buf, &face_camera, vw, vh);
                let dimmed = camera::dim(&cropped, factor);
                blit(&dimmed, face_area, &mut virtual_buf);
            }
        }
        let cam = Camera::new(self.displayed_face_index() * vw as f32, 0.0, 1.0);
        let view = camera::viewport(&virtual_buf, &cam, vw, vh);
        blit(&view, area, buf);

        let rotor_width = (area.width / 4).max(3);
        let rotor_area = Rect {
            x: area.x + area.width.saturating_sub(rotor_width) / 2,
            y: area.y + 1,
            width: rotor_width,
            height: area.height.saturating_sub(2),
        };
        TimeRotor::new(self.time_rotor_speed()).render(rotor_area, self.tick_count, buf);

        let hint_row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: area.height.saturating_sub(1).min(1),
        };
        Text::new("Left/Right rotate * Enter select * q quit").render(hint_row, buf);
    }
}
