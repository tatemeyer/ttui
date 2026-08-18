// examples/depth_spike.rs
//
// SPIKE PROTOTYPE for the depth & perspective projection spike
// (docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md).
// Not a themed vision-doc app — a bare showcase proving out real
// projection math. This file grows across that spec's implementation
// plan; expect prototype-quality code throughout.

use std::time::Duration;

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

const CUBE_HALF: f32 = 2.0;
const CUBE_MIN_Z: f32 = 4.0;
const CUBE_MAX_Z: f32 = 14.0;
const CUBE_DRIFT_SPEED: f32 = 2.0; // z-units/second

/// The 8 corners of a cube of half-width `CUBE_HALF` centered at
/// `(0, 0, center_z)`. Index order: `i = (dx_idx*2 + dy_idx)*2 + dz_idx`
/// for `dx_idx, dy_idx, dz_idx` each in `{0 (-), 1 (+)}`.
fn cube_vertices(center_z: f32) -> [Point3; 8] {
    let mut v = [Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }; 8];
    let mut i = 0;
    for dx in [-CUBE_HALF, CUBE_HALF] {
        for dy in [-CUBE_HALF, CUBE_HALF] {
            for dz in [-CUBE_HALF, CUBE_HALF] {
                v[i] = Point3 {
                    x: dx,
                    y: dy,
                    z: center_z + dz,
                };
                i += 1;
            }
        }
    }
    v
}

/// The cube's 12 edges as index pairs into `cube_vertices`'s output.
#[rustfmt::skip]
const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 4), (1, 5), (2, 6), (3, 7), // edges along dx
    (0, 2), (1, 3), (4, 6), (5, 7), // edges along dy
    (0, 1), (2, 3), (4, 5), (6, 7), // edges along dz
];

/// Just the near face's 4 edges — `cube_vertices` walks `dz` innermost,
/// so an even index is a `dz = -CUBE_HALF` (nearer) corner. Drawn
/// instead of all 12 once the cube is too far away for the full
/// wireframe to read as one (see `face_separation_cells`).
#[rustfmt::skip]
const NEAR_FACE_EDGES: [(usize, usize); 4] = [
    (0, 4), (2, 6), // along dx
    (0, 2), (4, 6), // along dy
];

/// Projected gap, in cells, between the cube's near and far faces at
/// `center_z`. Both project as rectangles about the same centre, so
/// this is just the difference of their half-widths.
///
/// Below one cell the two faces land on top of each other and all 12
/// edges smear into a filled band — the cube stops reading as a cube
/// and becomes a featureless blob (#121). That happens past
/// `center_z` ~ 11, matching the degradation the spike's own review
/// reported "past z ~ 10".
fn face_separation_cells(center_z: f32) -> f32 {
    let near = center_z - CUBE_HALF;
    let far = center_z + CUBE_HALF;
    if near <= NEAR_PLANE {
        return f32::INFINITY; // near face is at/behind the camera
    }
    CUBE_HALF * FOCAL_LENGTH * ASPECT_COMPENSATION * (1.0 / near - 1.0 / far)
}

/// Minimum face separation, in cells, for the full wireframe to still
/// read as a cube. Below it `render_cube` falls back to the near face
/// alone — an honest, legible outline rather than a smear. Deliberately
/// not a size *clamp*: the projection stays exact, since demonstrating
/// it is the whole point of this spike.
const MIN_FACE_SEPARATION_CELLS: f32 = 1.0;

const STAR_COUNT: usize = 80;
const STAR_SPEED: f32 = 4.0; // z-units/second
const STAR_RESPAWN_Z: f32 = 24.0;

/// One drifting star: a fixed `(x, y)` and a `z` that decreases over
/// time (drifting toward the camera), respawning far away once it
/// passes the near plane.
#[derive(Clone, Copy, Debug)]
struct Star {
    x: f32,
    y: f32,
    z: f32,
}

/// Deterministic pseudo-random scatter for star placement — no RNG
/// dependency, matching every prior Arc's posture (same style as
/// `src/glitch.rs`'s noise hash).
fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}

struct DepthSpike {
    stars: Vec<Star>,
    cube_z: f32,
    tick_count: u32,
    quit: bool,
}

impl DepthSpike {
    fn new() -> Self {
        let stars = (0..STAR_COUNT)
            .map(|i| {
                let seed = i as u32;
                Star {
                    x: scatter(seed, 16.0),
                    y: scatter(seed.wrapping_add(1_000), 10.0),
                    z: 2.0 + (seed as f32 % 20.0),
                }
            })
            .collect();
        DepthSpike {
            stars,
            cube_z: CUBE_MIN_Z,
            tick_count: 0,
            quit: false,
        }
    }

    fn render_starfield(&self, area: Rect, buf: &mut LayerStack) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        for star in &self.stars {
            let p = Point3 {
                x: star.x,
                y: star.y,
                z: star.z,
            };
            let Some((sx, sy, scale)) = project(p, center_x, center_y) else {
                continue;
            };
            let x = sx.round();
            let y = sy.round();
            if x < 0.0 || y < 0.0 || x as u16 >= area.width || y as u16 >= area.height {
                continue;
            }
            // Nearer (larger scale) stars render as a brighter, denser
            // glyph; farther ones as a dim, sparse glyph — both driven
            // by the same projection-derived `scale`, not a separate
            // hand-tuned depth curve.
            let symbol = if scale > 3.0 {
                '@'
            } else if scale > 1.5 {
                '*'
            } else {
                '.'
            };
            let brightness = (scale * 60.0).clamp(30.0, 255.0) as u8;
            buf.set(
                x as u16,
                y as u16,
                Cell {
                    symbol,
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

    fn render_cube(&self, area: Rect, buf: &mut LayerStack, center_z: f32) {
        let center_x = area.width as f32 / 2.0;
        let center_y = area.height as f32 / 2.0;
        let verts = cube_vertices(center_z);
        let mut canvas = Canvas::new(area.width, area.height, CanvasMode::Braille);

        // Front face (both dz=- corners: indices 0,2,6,4, walked as a
        // perimeter), filled first so the wireframe draws on top of it.
        let front_face = Polygon3 {
            vertices: vec![verts[0], verts[2], verts[6], verts[4]],
        };
        fill_polygon(
            &front_face,
            &mut canvas,
            center_x,
            center_y,
            2.0,
            4.0,
            Color::Rgb {
                r: 40,
                g: 60,
                b: 100,
            },
        );

        // LOD: too far for the full wireframe to be distinguishable ->
        // draw only the near face (#121).
        let edges: &[(usize, usize)] =
            if face_separation_cells(center_z) < MIN_FACE_SEPARATION_CELLS {
                &NEAR_FACE_EDGES
            } else {
                &CUBE_EDGES
            };

        for &(a, b) in edges {
            let line = Line3 {
                start: verts[a],
                end: verts[b],
            };
            if let Some((x0, y0, x1, y1)) = project_line(line, center_x, center_y, 2.0, 4.0) {
                canvas.line(
                    x0,
                    y0,
                    x1,
                    y1,
                    Color::Rgb {
                        r: 200,
                        g: 220,
                        b: 255,
                    },
                );
            }
        }

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
        self.render_starfield(area, buf);
        self.render_cube(area, buf, self.cube_z);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(33))
    }

    fn on_tick(&mut self, elapsed: Duration) {
        let dz = STAR_SPEED * elapsed.as_secs_f32();
        for (i, star) in self.stars.iter_mut().enumerate() {
            star.z -= dz;
            if star.z <= NEAR_PLANE {
                let seed = i as u32;
                star.z = STAR_RESPAWN_Z;
                star.x = scatter(seed.wrapping_add(self.tick_count), 16.0);
                star.y = scatter(seed.wrapping_add(self.tick_count).wrapping_add(1_000), 10.0);
            }
        }
        self.cube_z += CUBE_DRIFT_SPEED * elapsed.as_secs_f32();
        if self.cube_z > CUBE_MAX_Z {
            self.cube_z = CUBE_MIN_Z;
        }
        self.tick_count = self.tick_count.wrapping_add(1);
    }
}

fn main() -> std::io::Result<()> {
    let mut app = DepthSpike::new();
    run(&mut app)
}
