use super::*;

const HYPERDRIVE_DASH_COUNT: usize = 8;
const HYPERDRIVE_START: Point3 = Point3 {
    x: 0.0,
    y: 0.0,
    z: 2.5,
};
const HYPERDRIVE_END: Point3 = Point3 {
    x: 6.0,
    y: 2.0,
    z: 22.0,
};
const SENSOR_PLANE_Z: f32 = 6.0;
const SENSOR_RADIUS: f32 = 3.0;
const SENSOR_TRAIL_COUNT: usize = 4;
const SENSOR_TRAIL_STEP: f32 = 0.25; // radians between trailing lines
const WEAPONS_PLANE_Z: f32 = 5.0;
const WEAPONS_BASE_HALF_SIZE: f32 = 2.0;
const WEAPONS_PULSE_AMPLITUDE: f32 = 0.15;
const WEAPONS_BRACKET_LEN: f32 = 0.7;

impl Falcon {
    /// Projects `seg` and draws it onto `canvas` if it survives clipping —
    /// shared by every HUD render method so the subpixel clip-bound
    /// arithmetic (`area.width as f32 - 1.0/2.0`, `area.height as f32 -
    /// 1.0/4.0`) exists in exactly one place. That arithmetic is the fix
    /// for a real bug (canopy pillars silently culled at 80-column
    /// terminals) — see the doc comment on `render_canopy`'s call site.
    fn hud_line(&self, area: Rect, canvas: &mut Canvas, seg: Line3, color: Color) {
        if let Some((x0, y0, x1, y1)) = self.camera.project_line(
            seg,
            area.width as f32 / 2.0,
            area.height as f32 / 2.0,
            area.width as f32 - 1.0 / 2.0,
            area.height as f32 - 1.0 / 4.0,
            2.0,
            4.0,
            0.0,
        ) {
            canvas.line(x0, y0, x1, y1, color);
        }
    }

    fn render_hud_hyperdrive(&self, area: Rect, buf: &mut LayerStack) {
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        for i in 0..HYPERDRIVE_DASH_COUNT {
            let t0 = i as f32 / HYPERDRIVE_DASH_COUNT as f32;
            let t1 = (i + 1) as f32 / HYPERDRIVE_DASH_COUNT as f32;
            let seg = Line3 {
                start: Point3 {
                    x: lerp(HYPERDRIVE_START.x, HYPERDRIVE_END.x, t0),
                    y: lerp(HYPERDRIVE_START.y, HYPERDRIVE_END.y, t0),
                    z: lerp(HYPERDRIVE_START.z, HYPERDRIVE_END.z, t0),
                },
                end: Point3 {
                    x: lerp(HYPERDRIVE_START.x, HYPERDRIVE_END.x, t1),
                    y: lerp(HYPERDRIVE_START.y, HYPERDRIVE_END.y, t1),
                    z: lerp(HYPERDRIVE_START.z, HYPERDRIVE_END.z, t1),
                },
            };
            let phase_offset = i as f32 * (std::f32::consts::TAU / HYPERDRIVE_DASH_COUNT as f32);
            let brightness = 0.3 + 0.7 * (0.5 + 0.5 * (self.hyperdrive_phase - phase_offset).sin());
            let color =
                ttui::easing::lerp_color(self.theme.background, self.theme.accent, brightness);
            self.hud_line(area, &mut canvas, seg, color);
        }
        canvas.blit(buf, area.x, area.y);
    }

    fn render_hud_sensors(&self, area: Rect, buf: &mut LayerStack) {
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let center = Point3 {
            x: 0.0,
            y: 0.0,
            z: SENSOR_PLANE_Z,
        };
        for k in 0..=SENSOR_TRAIL_COUNT {
            let angle = self.sensor_sweep_angle - k as f32 * SENSOR_TRAIL_STEP;
            let tip = Point3 {
                x: SENSOR_RADIUS * angle.cos(),
                y: SENSOR_RADIUS * angle.sin(),
                z: SENSOR_PLANE_Z,
            };
            let brightness = 1.0 - (k as f32 / (SENSOR_TRAIL_COUNT + 1) as f32);
            let color =
                ttui::easing::lerp_color(self.theme.background, self.theme.secondary, brightness);
            let line = Line3 {
                start: center,
                end: tip,
            };
            self.hud_line(area, &mut canvas, line, color);
        }
        canvas.blit(buf, area.x, area.y);
    }

    fn render_hud_weapons(&self, area: Rect, buf: &mut LayerStack) {
        let half = WEAPONS_BASE_HALF_SIZE
            * (1.0 + WEAPONS_PULSE_AMPLITUDE * self.weapons_pulse_phase.sin());
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
        for &(cx, cy) in &corners {
            let dx = if cx < 0.0 {
                WEAPONS_BRACKET_LEN
            } else {
                -WEAPONS_BRACKET_LEN
            };
            let dy = if cy < 0.0 {
                WEAPONS_BRACKET_LEN
            } else {
                -WEAPONS_BRACKET_LEN
            };
            let corner = Point3 {
                x: cx,
                y: cy,
                z: WEAPONS_PLANE_Z,
            };
            let horiz = Line3 {
                start: corner,
                end: Point3 {
                    x: cx + dx,
                    y: cy,
                    z: WEAPONS_PLANE_Z,
                },
            };
            let vert = Line3 {
                start: corner,
                end: Point3 {
                    x: cx,
                    y: cy + dy,
                    z: WEAPONS_PLANE_Z,
                },
            };
            for seg in [horiz, vert] {
                self.hud_line(area, &mut canvas, seg, self.theme.tertiary);
            }
        }
        canvas.blit(buf, area.x, area.y);
    }

    pub(crate) fn render_hud(&self, area: Rect, buf: &mut LayerStack) {
        match PANELS[self.focused] {
            PanelKind::Hyperdrive => self.render_hud_hyperdrive(area, buf),
            PanelKind::Sensors => self.render_hud_sensors(area, buf),
            PanelKind::Weapons => self.render_hud_weapons(area, buf),
        }
    }
}
