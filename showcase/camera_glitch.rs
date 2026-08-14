//! Diagnostic Scan — a rotating 3D wireframe box (the gripper's arm
//! schematic), auto-glitching twice via GlitchBuffer; Space "whacks"
//! it clear early, mirroring falcon's percussive-maintenance mechanic
//! (examples/falcon/falcon.rs's FalconAction::Whack handler).

use std::time::Duration;
use ttui::buffer::LayerStack;
use ttui::canvas::{Canvas, CanvasMode};
use ttui::glitch::GlitchBuffer;
use ttui::layout::Rect;
use ttui::perspective::{Camera, Line3, Point3, ProjectLineParams};
use ttui::theme::Theme;

const ROTATION_SPEED: f32 = 1.2; // radians/second
const GLITCH_TRIGGER_AT: [Duration; 2] = [Duration::from_millis(1500), Duration::from_millis(3500)];
const GLITCH_DURATION: Duration = Duration::from_millis(600);
const BASE_Z: f32 = 6.0;

const CUBE_VERTS: [(f32, f32, f32); 8] = [
    (-1.0, -1.0, -1.0),
    (1.0, -1.0, -1.0),
    (1.0, 1.0, -1.0),
    (-1.0, 1.0, -1.0),
    (-1.0, -1.0, 1.0),
    (1.0, -1.0, 1.0),
    (1.0, 1.0, 1.0),
    (-1.0, 1.0, 1.0),
];
const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

fn rotate_y(v: (f32, f32, f32), angle: f32) -> Point3 {
    let (x, y, z) = v;
    Point3 {
        x: x * angle.cos() + z * angle.sin(),
        y,
        z: -x * angle.sin() + z * angle.cos() + BASE_Z,
    }
}

pub(crate) struct DiagnosticScanState {
    angle: f32,
    elapsed_total: Duration,
    glitch: GlitchBuffer,
    fired: [bool; 2],
    tick_count: u64,
}

impl DiagnosticScanState {
    pub(crate) fn new() -> Self {
        DiagnosticScanState {
            angle: 0.0,
            elapsed_total: Duration::ZERO,
            glitch: GlitchBuffer::new(),
            fired: [false, false],
            tick_count: 0,
        }
    }

    pub(crate) fn on_tick(&mut self, elapsed: Duration) {
        self.tick_count += 1;
        self.elapsed_total += elapsed;
        self.angle = (self.angle + ROTATION_SPEED * elapsed.as_secs_f32()) % std::f32::consts::TAU;
        self.glitch.tick(elapsed);
        for (i, &trigger_at) in GLITCH_TRIGGER_AT.iter().enumerate() {
            if !self.fired[i] && self.elapsed_total >= trigger_at {
                self.fired[i] = true;
                self.glitch.trigger(GLITCH_DURATION);
            }
        }
    }

    /// Clears an active glitch early — the "percussive maintenance"
    /// mechanic, same shape as falcon's Whack handler.
    pub(crate) fn whack(&mut self) {
        if self.glitch.is_active() {
            self.glitch.clear();
        }
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.fired[1] && !self.glitch.is_active()
    }

    pub(crate) fn render(&self, area: Rect, theme: &Theme, buf: &mut LayerStack) {
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let cam = Camera {
            near: 0.5,
            focal_length: 8.0,
        };
        let params = ProjectLineParams {
            center_x: area.width as f32 / 2.0,
            center_y: area.height as f32 / 2.0,
            screen_w: area.width as f32 - 1.0 / 2.0,
            screen_h: area.height as f32 - 1.0 / 4.0,
            subpixels_x: 2.0,
            subpixels_y: 4.0,
            min_scale: 0.0,
        };
        for &(a, b) in CUBE_EDGES.iter() {
            let start = rotate_y(CUBE_VERTS[a], self.angle);
            let end = rotate_y(CUBE_VERTS[b], self.angle);
            if let Some((x0, y0, x1, y1)) = cam.project_line(Line3 { start, end }, params) {
                canvas.line(x0, y0, x1, y1, theme.primary);
            }
        }
        canvas.blit(buf, area.x, area.y);
        if self.glitch.is_active() {
            let overlay = buf.push_layer();
            self.glitch
                .render(area, theme.tertiary, self.tick_count, overlay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_glitch_fires_at_its_trigger_time() {
        let mut s = DiagnosticScanState::new();
        s.on_tick(GLITCH_TRIGGER_AT[0]);
        assert!(s.glitch.is_active());
        assert!(s.fired[0]);
        assert!(!s.fired[1]);
    }

    #[test]
    fn whack_clears_an_active_glitch() {
        let mut s = DiagnosticScanState::new();
        s.on_tick(GLITCH_TRIGGER_AT[0]);
        assert!(s.glitch.is_active());
        s.whack();
        assert!(!s.glitch.is_active());
    }

    #[test]
    fn whack_on_an_inactive_glitch_is_a_no_op() {
        let mut s = DiagnosticScanState::new();
        s.whack(); // nothing active yet
        assert!(!s.glitch.is_active());
    }

    #[test]
    fn is_complete_only_after_the_second_glitch_clears() {
        let mut s = DiagnosticScanState::new();
        s.on_tick(GLITCH_TRIGGER_AT[0]);
        assert!(!s.is_complete());
        s.whack();
        assert!(
            !s.is_complete(),
            "first glitch cleared, second hasn't fired yet"
        );
        s.on_tick(GLITCH_TRIGGER_AT[1] - GLITCH_TRIGGER_AT[0]);
        assert!(s.fired[1]);
        assert!(!s.is_complete(), "second glitch just fired, still active");
        s.whack();
        assert!(s.is_complete());
    }

    #[test]
    fn angle_advances_with_elapsed_time() {
        let mut s = DiagnosticScanState::new();
        let elapsed = Duration::from_millis(500);
        s.on_tick(elapsed);
        let expected = (ROTATION_SPEED * elapsed.as_secs_f32()) % std::f32::consts::TAU;
        assert!(
            (s.angle - expected).abs() < 0.001,
            "expected angle {expected}, got {}",
            s.angle
        );
    }
}
