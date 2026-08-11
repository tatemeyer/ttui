//! Fixed-forward pinhole-camera projection — points, lines, and
//! polygons in camera-relative 3D space project to 2D screen
//! coordinates plus a depth-derived scale. Graduated from the depth &
//! perspective projection spike
//! (docs/design/specs/core/2026-08-10-depth-perspective-projection-spike-design.md)
//! per
//! docs/design/specs/core/2026-08-11-perspective-projection-graduation-design.md.
//! No camera position/orientation — see that spec's Non-goals.

/// A point in camera-relative 3D space.
#[derive(Clone, Copy, Debug)]
pub struct Point3 {
    /// Horizontal position, camera-relative.
    pub x: f32,
    /// Vertical position, camera-relative.
    pub y: f32,
    /// Depth (distance along the camera's forward axis).
    pub z: f32,
}

/// A line segment in camera-relative 3D space.
#[derive(Clone, Copy, Debug)]
pub struct Line3 {
    /// The segment's first endpoint.
    pub start: Point3,
    /// The segment's second endpoint.
    pub end: Point3,
}

/// A flat-shaded polygon in camera-relative 3D space — 3+ vertices,
/// in order around the perimeter (not necessarily convex).
#[derive(Clone, Debug)]
pub struct Polygon3 {
    /// Vertices in order around the perimeter.
    pub vertices: Vec<Point3>,
}

/// A fixed-forward pinhole camera: positioned at the origin, looking
/// down `+Z`. No position/orientation fields — general camera
/// movement is out of scope.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Points at or nearer than this depth are clipped.
    pub near: f32,
    /// Controls field of view: larger = more zoomed in/narrower FOV.
    pub focal_length: f32,
}

/// Corrects for terminal cells being roughly twice as tall as wide —
/// same compensation the `Dial` widget already applies to its ring
/// radius.
pub const ASPECT_COMPENSATION: f32 = 2.0;

impl Camera {
    /// Projects `p` to `(screen_x, screen_y, scale)` in cell
    /// coordinates relative to `(center_x, center_y)`, where `scale`
    /// grows as the point gets nearer (drives size/brightness
    /// falloff). Returns `None` if `p.z <= self.near`.
    pub fn project(&self, p: Point3, center_x: f32, center_y: f32) -> Option<(f32, f32, f32)> {
        if p.z <= self.near {
            return None;
        }
        let ndc_x = p.x / p.z;
        let ndc_y = p.y / p.z;
        let scale = self.focal_length / p.z;
        let screen_x = center_x + ndc_x * self.focal_length * ASPECT_COMPENSATION;
        let screen_y = center_y - ndc_y * self.focal_length;
        Some((screen_x, screen_y, scale))
    }
}

const OUTCODE_INSIDE: u8 = 0;
const OUTCODE_LEFT: u8 = 1;
const OUTCODE_RIGHT: u8 = 2;
const OUTCODE_TOP: u8 = 4;
const OUTCODE_BOTTOM: u8 = 8;

fn outcode(x: f32, y: f32, xmax: f32, ymax: f32) -> u8 {
    let mut code = OUTCODE_INSIDE;
    if x < 0.0 {
        code |= OUTCODE_LEFT;
    } else if x > xmax {
        code |= OUTCODE_RIGHT;
    }
    if y < 0.0 {
        code |= OUTCODE_TOP;
    } else if y > ymax {
        code |= OUTCODE_BOTTOM;
    }
    code
}

/// Clips a line segment to the visible `[0, xmax] x [0, ymax]`
/// rectangle via Cohen-Sutherland. Returns `None` if the segment lies
/// entirely outside.
fn clip_to_screen(
    mut x0: f32,
    mut y0: f32,
    mut x1: f32,
    mut y1: f32,
    xmax: f32,
    ymax: f32,
) -> Option<(f32, f32, f32, f32)> {
    let mut code0 = outcode(x0, y0, xmax, ymax);
    let mut code1 = outcode(x1, y1, xmax, ymax);
    loop {
        if code0 | code1 == 0 {
            return Some((x0, y0, x1, y1));
        }
        if code0 & code1 != 0 {
            return None;
        }
        let out = if code0 != 0 { code0 } else { code1 };
        let (x, y) = if out & OUTCODE_TOP != 0 {
            (x0 + (x1 - x0) * (0.0 - y0) / (y1 - y0), 0.0)
        } else if out & OUTCODE_BOTTOM != 0 {
            (x0 + (x1 - x0) * (ymax - y0) / (y1 - y0), ymax)
        } else if out & OUTCODE_RIGHT != 0 {
            (xmax, y0 + (y1 - y0) * (xmax - x0) / (x1 - x0))
        } else {
            (0.0, y0 + (y1 - y0) * (0.0 - x0) / (x1 - x0))
        };
        if out == code0 {
            x0 = x;
            y0 = y;
            code0 = outcode(x0, y0, xmax, ymax);
        } else {
            x1 = x;
            y1 = y;
            code1 = outcode(x1, y1, xmax, ymax);
        }
    }
}

impl Camera {
    /// Projects `line` to `Canvas` subpixel coordinates for a canvas
    /// whose subpixel grid is `subpixels_x`/`subpixels_y` per cell and
    /// `screen_w`/`screen_h` cells wide/tall. Returns `None` if either
    /// endpoint is at/behind the near plane, if every vertex's `scale`
    /// is below `min_scale`, or if the projected segment falls
    /// entirely outside the visible screen. A segment partially
    /// outside is clipped to the visible rectangle before being
    /// converted to subpixel coordinates.
    #[allow(clippy::too_many_arguments)]
    pub fn project_line(
        &self,
        line: Line3,
        center_x: f32,
        center_y: f32,
        screen_w: f32,
        screen_h: f32,
        subpixels_x: f32,
        subpixels_y: f32,
        min_scale: f32,
    ) -> Option<(u16, u16, u16, u16)> {
        let (sx0, sy0, scale0) = self.project(line.start, center_x, center_y)?;
        let (sx1, sy1, scale1) = self.project(line.end, center_x, center_y)?;
        if scale0.max(scale1) < min_scale {
            return None;
        }
        let (cx0, cy0, cx1, cy1) = clip_to_screen(sx0, sy0, sx1, sy1, screen_w, screen_h)?;
        Some((
            (cx0 * subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
            (cy0 * subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
            (cx1 * subpixels_x).round().clamp(0.0, u16::MAX as f32) as u16,
            (cy1 * subpixels_y).round().clamp(0.0, u16::MAX as f32) as u16,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Camera {
        Camera {
            near: 0.5,
            focal_length: 8.0,
        }
    }

    #[test]
    fn point_behind_the_near_plane_returns_none() {
        let cam = camera();
        let p = Point3 {
            x: 4.0,
            y: 0.0,
            z: 0.2,
        };
        assert_eq!(cam.project(p, 10.0, 5.0), None);
    }

    #[test]
    fn point_exactly_at_the_near_plane_returns_none() {
        let cam = camera();
        let p = Point3 {
            x: 4.0,
            y: 0.0,
            z: 0.5,
        };
        assert_eq!(cam.project(p, 10.0, 5.0), None);
    }

    #[test]
    fn nearer_points_project_farther_from_center_and_with_larger_scale() {
        let cam = camera();
        let center_x = 10.0;
        let center_y = 5.0;

        let (x2, y2, s2) = cam
            .project(
                Point3 {
                    x: 4.0,
                    y: 0.0,
                    z: 2.0,
                },
                center_x,
                center_y,
            )
            .unwrap();
        let (x4, y4, s4) = cam
            .project(
                Point3 {
                    x: 4.0,
                    y: 0.0,
                    z: 4.0,
                },
                center_x,
                center_y,
            )
            .unwrap();
        let (x8, y8, s8) = cam
            .project(
                Point3 {
                    x: 4.0,
                    y: 0.0,
                    z: 8.0,
                },
                center_x,
                center_y,
            )
            .unwrap();

        // Hand-verified (all inputs are powers of 2, so every
        // intermediate value is exactly representable in binary
        // floating point — no epsilon needed):
        // z=2: ndc_x=2.0, scale=4.0, screen_x=10+2.0*8*2=42.0
        // z=4: ndc_x=1.0, scale=2.0, screen_x=10+1.0*8*2=26.0
        // z=8: ndc_x=0.5, scale=1.0, screen_x=10+0.5*8*2=18.0
        assert_eq!((x2, s2), (42.0, 4.0));
        assert_eq!((x4, s4), (26.0, 2.0));
        assert_eq!((x8, s8), (18.0, 1.0));
        assert_eq!(y2, 5.0);
        assert_eq!(y4, 5.0);
        assert_eq!(y8, 5.0);

        // Farther points land closer to center_x (convergence toward
        // the vanishing point) and have smaller scale (dimmer/smaller).
        assert!((x8 - center_x).abs() < (x4 - center_x).abs());
        assert!((x4 - center_x).abs() < (x2 - center_x).abs());
        assert!(s8 < s4 && s4 < s2);
    }

    #[test]
    fn positive_world_y_maps_to_a_smaller_screen_y() {
        let cam = camera();
        // z=4.0, y=2.0: ndc_y=0.5, screen_y=5.0-0.5*8.0=1.0 (exact).
        let (x, y, _) = cam
            .project(
                Point3 {
                    x: 0.0,
                    y: 2.0,
                    z: 4.0,
                },
                10.0,
                5.0,
            )
            .unwrap();
        assert_eq!(x, 10.0); // ndc_x = 0 -> no horizontal offset
        assert_eq!(y, 1.0);
    }

    fn assert_close(actual: f32, expected: f32, label: &str) {
        assert!(
            (actual - expected).abs() < 0.001,
            "{label}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn clip_to_screen_leaves_a_fully_inside_line_unchanged() {
        let result = clip_to_screen(2.0, 2.0, 8.0, 8.0, 10.0, 10.0);
        assert_eq!(result, Some((2.0, 2.0, 8.0, 8.0)));
    }

    #[test]
    fn clip_to_screen_rejects_a_line_fully_outside_on_each_side() {
        assert_eq!(clip_to_screen(-5.0, 3.0, -1.0, 3.0, 10.0, 10.0), None); // left
        assert_eq!(clip_to_screen(15.0, 3.0, 20.0, 3.0, 10.0, 10.0), None); // right
        assert_eq!(clip_to_screen(3.0, -5.0, 3.0, -1.0, 10.0, 10.0), None); // top
        assert_eq!(clip_to_screen(3.0, 15.0, 3.0, 20.0, 10.0, 10.0), None); // bottom
    }

    #[test]
    fn clip_to_screen_clips_a_single_edge_crossing() {
        // (5,5) is inside; (15,5) is outside past the right edge.
        let result = clip_to_screen(5.0, 5.0, 15.0, 5.0, 10.0, 10.0);
        assert_eq!(result, Some((5.0, 5.0, 10.0, 5.0)));
    }

    #[test]
    fn clip_to_screen_resolves_a_two_edge_corner_crossing() {
        // (20,15) is outside past both the right and bottom edges;
        // Cohen-Sutherland must resolve this through two clip
        // iterations (an intermediate point against one edge that's
        // still outside via the other) before landing inside.
        let result = clip_to_screen(20.0, 15.0, 5.0, 5.0, 10.0, 10.0);
        let (cx0, cy0, cx1, cy1) = result.expect("line intersects the box");
        assert_close(cx0, 10.0, "clipped start.x (right edge)");
        assert!(
            (0.0..=10.0).contains(&cy0),
            "clipped start.y should be within the box, got {cy0}"
        );
        assert_eq!(cx1, 5.0);
        assert_eq!(cy1, 5.0);
    }

    #[test]
    fn project_line_returns_none_when_either_endpoint_is_behind_the_near_plane() {
        let cam = camera();
        let line = Line3 {
            start: Point3 {
                x: -1.0,
                y: 0.0,
                z: 4.0,
            },
            end: Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.2,
            },
        };
        assert_eq!(
            cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0),
            None
        );
    }

    #[test]
    fn project_line_returns_none_when_every_vertexs_scale_is_below_min_scale() {
        let cam = camera();
        // Both points at z=100: scale = 8.0/100.0 = 0.08.
        let line = Line3 {
            start: Point3 {
                x: -1.0,
                y: 0.0,
                z: 100.0,
            },
            end: Point3 {
                x: 1.0,
                y: 0.0,
                z: 100.0,
            },
        };
        assert_eq!(
            cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.1),
            None
        );
    }

    #[test]
    fn project_line_projects_a_fully_visible_line_to_subpixel_coordinates() {
        let cam = camera();
        // Both points at z=4.0: x=-1 -> ndc_x=-0.25 -> screen_x=5+(-0.25*8*2)=1.0
        //                        x=1  -> ndc_x=0.25  -> screen_x=5+(0.25*8*2)=9.0
        // Both within [0,10]x[0,10] -> no clipping. Subpixel factors (2.0,4.0):
        // (1.0*2, 5.0*4, 9.0*2, 5.0*4) = (2, 20, 18, 20).
        let line = Line3 {
            start: Point3 {
                x: -1.0,
                y: 0.0,
                z: 4.0,
            },
            end: Point3 {
                x: 1.0,
                y: 0.0,
                z: 4.0,
            },
        };
        let result = cam.project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0);
        assert_eq!(result, Some((2, 20, 18, 20)));
    }

    #[test]
    fn project_line_clips_instead_of_saturating_an_off_screen_endpoint() {
        let cam = camera();
        // x=20 at z=4.0: ndc_x=5.0, screen_x=5+5.0*8*2=85.0 (far off-screen).
        // x=0 at z=4.0: screen_x=5.0 (on-screen, at y=5.0 both).
        // Real clipping lands the off-screen endpoint at screen_x=10.0
        // (the right edge), not saturated to 0 (the spike's bug).
        let line = Line3 {
            start: Point3 {
                x: 20.0,
                y: 0.0,
                z: 4.0,
            },
            end: Point3 {
                x: 0.0,
                y: 0.0,
                z: 4.0,
            },
        };
        let result = cam
            .project_line(line, 5.0, 5.0, 10.0, 10.0, 2.0, 4.0, 0.0)
            .expect("line crosses into the visible screen");
        // Clipped start lands at screen (10.0, 5.0) -> subpixel (20, 20).
        assert_eq!(result, (20, 20, 10, 20));
    }
}
