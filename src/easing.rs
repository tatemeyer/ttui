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
}
