//! Linear/eased interpolation and progress helpers — the building
//! blocks every `Transition`-driven animation in this codebase uses.

use crossterm::style::Color;
use std::time::Duration;

/// Linear interpolation from `start` to `end`, `t` clamped to `0..1`.
pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    start + (end - start) * t
}

/// Interpolation from `start` to `end` that starts fast and eases
/// into `end`, `t` clamped to `0..1`.
pub fn ease_out(start: f32, end: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    lerp(start, end, eased)
}

/// `elapsed / duration`, clamped to `0..1`; a zero `duration` is
/// always complete.
pub fn progress(elapsed: Duration, duration: Duration) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

/// Color lerp from `from` to `to`, `t` clamped to `0..1`.
///
/// An `Rgb` pair interpolates componentwise. Any other pair cannot —
/// `Color::Reset` means "whatever the terminal's default is" and named
/// colors are whatever the terminal's theme maps them to, so there is
/// no honest RGB value to interpolate through, and inventing one would
/// emit a shade the terminal did not choose. Such a pair therefore
/// switches at the midpoint: `from` below `0.5`, `to` at or above.
///
/// That switch is a visible step rather than a ramp, but it keeps the
/// endpoints honest, which is what actually matters — the previous
/// fallback returned `to` for every `t`, so a gradient rendered flat at
/// its end color and a fade jumped to its target on the first frame
/// (#122).
pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    match (from, to) {
        (
            Color::Rgb {
                r: r1,
                g: g1,
                b: b1,
            },
            Color::Rgb {
                r: r2,
                g: g2,
                b: b2,
            },
        ) => Color::Rgb {
            r: lerp(r1 as f32, r2 as f32, t) as u8,
            g: lerp(g1 as f32, g2 as f32, t) as u8,
            b: lerp(b1 as f32, b2 as f32, t) as u8,
        },
        _ => {
            if t < 0.5 {
                from
            } else {
                to
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp_endpoints() {
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
    }

    #[test]
    fn test_lerp_midpoint() {
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
    }

    #[test]
    fn test_lerp_clamps_below() {
        assert_eq!(lerp(0.0, 10.0, -1.0), 0.0);
    }

    #[test]
    fn test_lerp_clamps_above() {
        assert_eq!(lerp(0.0, 10.0, 2.0), 10.0);
    }

    #[test]
    fn test_ease_out_endpoints() {
        assert_eq!(ease_out(0.0, 10.0, 0.0), 0.0);
        assert_eq!(ease_out(0.0, 10.0, 1.0), 10.0);
    }

    #[test]
    fn test_ease_out_faster_than_linear() {
        let linear = lerp(0.0, 10.0, 0.5);
        let eased = ease_out(0.0, 10.0, 0.5);
        assert!(eased > linear);
    }

    #[test]
    fn test_progress_zero_elapsed() {
        assert_eq!(progress(Duration::ZERO, Duration::from_secs(1)), 0.0);
    }

    #[test]
    fn test_progress_equal_duration() {
        assert_eq!(
            progress(Duration::from_secs(1), Duration::from_secs(1)),
            1.0
        );
    }

    #[test]
    fn test_progress_halfway() {
        assert_eq!(
            progress(Duration::from_millis(500), Duration::from_secs(1)),
            0.5
        );
    }

    #[test]
    fn test_progress_elapsed_exceeds_duration() {
        assert_eq!(
            progress(Duration::from_secs(2), Duration::from_secs(1)),
            1.0
        );
    }

    #[test]
    fn test_progress_zero_duration() {
        assert_eq!(progress(Duration::from_secs(1), Duration::ZERO), 1.0);
    }

    #[test]
    fn test_lerp_color_endpoints() {
        let from = Color::Rgb { r: 0, g: 0, b: 0 };
        let to = Color::Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        assert_eq!(lerp_color(from, to, 0.0), from);
        assert_eq!(lerp_color(from, to, 1.0), to);
    }

    #[test]
    fn test_lerp_color_midpoint() {
        let from = Color::Rgb { r: 0, g: 0, b: 0 };
        let to = Color::Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        assert_eq!(
            lerp_color(from, to, 0.5),
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
    }

    #[test]
    fn test_lerp_color_non_rgb_falls_back_to_target() {
        let from = Color::Reset;
        let to = Color::Rgb {
            r: 10,
            g: 20,
            b: 30,
        };
        assert_eq!(lerp_color(from, to, 0.5), to);
    }

    /// #122: the fallback used to return `to` for every `t`, so a lerp
    /// involving a non-Rgb color never showed its *source* — a gradient
    /// rendered flat at its end color, and a fade snapped to its target
    /// on the first frame. `t` is now respected at the endpoints even
    /// when the colors cannot be interpolated componentwise.
    #[test]
    fn test_lerp_color_non_rgb_endpoints_are_the_endpoints() {
        let from = Color::Green;
        let to = Color::Red;

        assert_eq!(lerp_color(from, to, 0.0), from, "t=0 must be `from`");
        assert_eq!(lerp_color(from, to, 1.0), to, "t=1 must be `to`");
        // Below the midpoint still reads as `from`, at or above as `to`.
        assert_eq!(lerp_color(from, to, 0.25), from);
        assert_eq!(lerp_color(from, to, 0.75), to);
    }

    /// A pair that *can* be interpolated is unaffected by the above.
    #[test]
    fn test_lerp_color_rgb_pair_still_interpolates_componentwise() {
        let from = Color::Rgb { r: 0, g: 0, b: 0 };
        let to = Color::Rgb {
            r: 100,
            g: 200,
            b: 40,
        };
        assert_eq!(
            lerp_color(from, to, 0.5),
            Color::Rgb {
                r: 50,
                g: 100,
                b: 20
            }
        );
    }
}
