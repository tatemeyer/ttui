use crossterm::style::Color;
use std::time::Duration;

pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    start + (end - start) * t
}

pub fn ease_out(start: f32, end: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    lerp(start, end, eased)
}

pub fn progress(elapsed: Duration, duration: Duration) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

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
        _ => to,
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
}
