use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    duration: Duration,
    elapsed: Duration,
}

impl Transition {
    pub fn start(duration: Duration) -> Self {
        Transition {
            duration,
            elapsed: Duration::ZERO,
        }
    }

    pub fn tick(&mut self, elapsed: Duration) {
        self.elapsed = (self.elapsed + elapsed).min(self.duration);
    }

    pub fn progress(&self) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (self.elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    pub fn is_complete(&self) -> bool {
        self.elapsed >= self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_starts_at_zero_progress() {
        let transition = Transition::start(Duration::from_secs(1));
        assert_eq!(transition.progress(), 0.0);
        assert!(!transition.is_complete());
    }

    #[test]
    fn tick_advances_progress_proportionally() {
        let mut transition = Transition::start(Duration::from_secs(1));
        transition.tick(Duration::from_millis(250));
        assert_eq!(transition.progress(), 0.25);
    }

    #[test]
    fn tick_accumulates_across_multiple_calls() {
        let mut transition = Transition::start(Duration::from_secs(1));
        transition.tick(Duration::from_millis(250));
        transition.tick(Duration::from_millis(250));
        assert_eq!(transition.progress(), 0.5);
    }

    #[test]
    fn tick_clamps_progress_at_one() {
        let mut transition = Transition::start(Duration::from_millis(100));
        transition.tick(Duration::from_millis(200));
        assert_eq!(transition.progress(), 1.0);
    }

    #[test]
    fn is_complete_becomes_true_after_duration() {
        let mut transition = Transition::start(Duration::from_millis(100));
        assert!(!transition.is_complete());
        transition.tick(Duration::from_millis(100));
        assert!(transition.is_complete());
    }

    #[test]
    fn zero_duration_is_immediately_complete() {
        let transition = Transition::start(Duration::ZERO);
        assert_eq!(transition.progress(), 1.0);
        assert!(transition.is_complete());
    }
}
