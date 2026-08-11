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
}
