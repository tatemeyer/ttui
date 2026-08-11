// examples/depth_spike.rs
//
// SPIKE PROTOTYPE for the depth & perspective projection spike
// (docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md).
// Not a themed vision-doc app — a bare showcase proving out real
// projection math. This file grows across that spec's implementation
// plan; expect prototype-quality code throughout.

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::style::Color;
use ttui::app::{run, App};
use ttui::buffer::{Cell, LayerStack};
use ttui::layout::Rect;

/// Camera fixed at the origin, looking down `+Z` — no position/
/// orientation changes this Arc (confirmed fixed-forward only).
const NEAR_PLANE: f32 = 0.5;
/// Controls field of view: larger = more zoomed in/narrower FOV.
const FOCAL_LENGTH: f32 = 8.0;
/// Corrects for terminal cells being roughly twice as tall as wide —
/// same compensation the `Dial` widget already applies to its ring
/// radius.
const ASPECT_COMPENSATION: f32 = 2.0;

/// A point in camera-relative 3D space.
#[derive(Clone, Copy, Debug)]
struct Point3 {
    x: f32,
    y: f32,
    z: f32,
}

/// Projects `p` to `(screen_x, screen_y, scale)` in cell coordinates,
/// where `scale` grows as the point gets nearer (drives size/
/// brightness falloff). Returns `None` if `p.z <= NEAR_PLANE` — behind
/// or at the camera, where projection is undefined/inverted.
fn project(p: Point3, center_x: f32, center_y: f32) -> Option<(f32, f32, f32)> {
    if p.z <= NEAR_PLANE {
        return None;
    }
    let ndc_x = p.x / p.z;
    let ndc_y = p.y / p.z;
    let scale = FOCAL_LENGTH / p.z;
    let screen_x = center_x + ndc_x * FOCAL_LENGTH * ASPECT_COMPENSATION;
    let screen_y = center_y - ndc_y * FOCAL_LENGTH;
    Some((screen_x, screen_y, scale))
}

struct DepthSpike {
    quit: bool,
}

impl DepthSpike {
    fn new() -> Self {
        DepthSpike { quit: false }
    }
}

impl App for DepthSpike {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        if k.code == KeyCode::Char('q') {
            self.quit = true;
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        // Sanity check: four points at increasing depth (plus one
        // behind the near plane, which must not render) along the
        // same x/y — nearer ones should land farther from center and
        // brighter; farther ones should converge toward center and
        // dim; the z=0.2 point must not appear at all.
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let test_points = [
            Point3 { x: 3.0, y: 0.0, z: 2.0 },
            Point3 { x: 3.0, y: 0.0, z: 5.0 },
            Point3 { x: 3.0, y: 0.0, z: 10.0 },
            Point3 { x: 3.0, y: 0.0, z: 0.2 },
        ];
        for p in test_points {
            let Some((sx, sy, scale)) = project(p, center_x, center_y) else {
                continue;
            };
            let x = sx.round();
            let y = sy.round();
            if x < 0.0 || y < 0.0 || x as u16 >= area.width || y as u16 >= area.height {
                continue;
            }
            let brightness = (scale * 40.0).clamp(40.0, 255.0) as u8;
            buf.set(
                x as u16,
                y as u16,
                Cell {
                    symbol: '*',
                    fg: Color::Rgb {
                        r: brightness,
                        g: brightness,
                        b: brightness,
                    },
                    bg: Color::Reset,
                    alpha: 1.0,
                    ..Default::default()
                },
            );
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

fn main() -> std::io::Result<()> {
    let mut app = DepthSpike::new();
    run(&mut app)
}
