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
use ttui::canvas::{Canvas, CanvasMode};
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

/// A line segment in camera-relative 3D space.
#[derive(Clone, Copy, Debug)]
struct Line3 {
    start: Point3,
    end: Point3,
}

/// Projects `line` to `Canvas` subpixel coordinates for a canvas whose
/// subpixel grid is `subpixels_x`/`subpixels_y` per cell. Simplified
/// clipping for this spike: `None` if *either* endpoint is at/behind
/// the near plane — true near-plane segment clipping (computing the
/// intersection point) is a documented Non-goal unless this spike
/// finds it's actually needed.
fn project_line(
    line: Line3,
    center_x: f32,
    center_y: f32,
    subpixels_x: f32,
    subpixels_y: f32,
) -> Option<(u16, u16, u16, u16)> {
    let (sx0, sy0, _) = project(line.start, center_x, center_y)?;
    let (sx1, sy1, _) = project(line.end, center_x, center_y)?;
    Some((
        (sx0 * subpixels_x).round().max(0.0) as u16,
        (sy0 * subpixels_y).round().max(0.0) as u16,
        (sx1 * subpixels_x).round().max(0.0) as u16,
        (sy1 * subpixels_y).round().max(0.0) as u16,
    ))
}

/// A flat-shaded polygon in camera-relative 3D space — 3+ vertices,
/// in order around the perimeter (not necessarily convex).
#[derive(Clone, Debug)]
struct Polygon3 {
    vertices: Vec<Point3>,
}

/// Projects and fills `polygon` into `canvas` via an even-odd scanline
/// fill (one horizontal span per canvas-subpixel row, between each
/// pair of edge crossings). Draws nothing if any vertex is at/behind
/// the near plane (same simplified clipping as `project_line` — any
/// one clipped vertex skips the whole polygon) or if fewer than 3
/// vertices remain.
fn fill_polygon(
    polygon: &Polygon3,
    canvas: &mut Canvas,
    center_x: f32,
    center_y: f32,
    subpixels_x: f32,
    subpixels_y: f32,
    color: Color,
) {
    let mut points: Vec<(f32, f32)> = Vec::with_capacity(polygon.vertices.len());
    for &v in &polygon.vertices {
        let Some((sx, sy, _)) = project(v, center_x, center_y) else {
            return;
        };
        points.push((sx * subpixels_x, sy * subpixels_y));
    }
    if points.len() < 3 {
        return;
    }
    let min_y = points
        .iter()
        .map(|p| p.1)
        .fold(f32::INFINITY, f32::min)
        .floor() as i32;
    let max_y = points
        .iter()
        .map(|p| p.1)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil() as i32;
    for y in min_y.max(0)..=max_y {
        let yf = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::new();
        for i in 0..points.len() {
            let (x0, y0) = points[i];
            let (x1, y1) = points[(i + 1) % points.len()];
            if (y0 <= yf && y1 > yf) || (y1 <= yf && y0 > yf) {
                let t = (yf - y0) / (y1 - y0);
                xs.push(x0 + t * (x1 - x0));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut i = 0;
        while i + 1 < xs.len() {
            let x_start = xs[i].round().max(0.0) as u16;
            let x_end = xs[i + 1].round().max(0.0) as u16;
            for x in x_start..=x_end {
                canvas.set_pixel(x, y as u16, color);
            }
            i += 2;
        }
    }
}

struct DepthSpike {
    quit: bool,
}

impl DepthSpike {
    fn new() -> Self {
        DepthSpike { quit: false }
    }

    fn render_test_lines(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let lines = [
            Line3 {
                start: Point3 { x: -3.0, y: -2.0, z: 4.0 },
                end: Point3 { x: 3.0, y: -2.0, z: 4.0 },
            },
            Line3 {
                start: Point3 { x: -3.0, y: 2.0, z: 4.0 },
                end: Point3 { x: 3.0, y: 2.0, z: 4.0 },
            },
            Line3 {
                start: Point3 { x: -3.0, y: -2.0, z: 4.0 },
                end: Point3 { x: -5.0, y: -3.0, z: 10.0 },
            },
            Line3 {
                start: Point3 { x: 3.0, y: -2.0, z: 4.0 },
                end: Point3 { x: 5.0, y: -3.0, z: 10.0 },
            },
        ];
        for line in lines {
            if let Some((x0, y0, x1, y1)) =
                project_line(line, center_x, center_y, 2.0, 4.0)
            {
                canvas.line(x0, y0, x1, y1, Color::Rgb { r: 90, g: 180, b: 255 });
            }
        }
        canvas.blit(buf, area.x, area.y);
    }

    fn render_test_polygon(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);
        let polygon = Polygon3 {
            vertices: vec![
                Point3 { x: -4.0, y: -2.5, z: 6.0 },
                Point3 { x: -4.0, y: 2.5, z: 6.0 },
                Point3 { x: 2.0, y: 3.0, z: 12.0 },
                Point3 { x: 2.0, y: -3.0, z: 12.0 },
            ],
        };
        fill_polygon(
            &polygon,
            &mut canvas,
            center_x,
            center_y,
            2.0,
            4.0,
            Color::Rgb { r: 60, g: 40, b: 100 },
        );
        canvas.blit(buf, area.x, area.y);
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
        self.render_test_lines(area, buf);
        self.render_test_polygon(area, buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

fn main() -> std::io::Result<()> {
    let mut app = DepthSpike::new();
    run(&mut app)
}
