//! Time-driven progress tracking for animations — apps own a
//! `Transition` per animated value, `tick` it each frame, and read
//! `progress()`/`is_complete()` to drive rendering.

use std::time::Duration;

/// Tracks elapsed time against a fixed duration, exposing progress as
/// `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    duration: Duration,
    elapsed: Duration,
}

impl Transition {
    /// Starts a transition that completes after `duration`.
    pub fn start(duration: Duration) -> Self {
        Transition {
            duration,
            elapsed: Duration::ZERO,
        }
    }

    /// Advances elapsed time by `elapsed`, clamped to `duration`.
    pub fn tick(&mut self, elapsed: Duration) {
        self.elapsed = (self.elapsed + elapsed).min(self.duration);
    }

    /// Elapsed / duration, clamped to `0.0..=1.0`. A zero-duration
    /// transition is always `1.0`.
    pub fn progress(&self) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (self.elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Whether elapsed time has reached `duration`.
    pub fn is_complete(&self) -> bool {
        self.elapsed >= self.duration
    }
}

/// Subdivides a `0..1` progress range into `N` phases.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phases<const N: usize> {
    ends: [f32; N],
}

impl<const N: usize> Phases<N> {
    /// Builds phases from cumulative phase *ends* in ascending order,
    /// the last being `1.0`.
    pub const fn new(ends: [f32; N]) -> Self {
        Phases { ends }
    }

    /// The phase `progress` falls in, and how far through that phase it
    /// is — always clamped to `0..1`.
    pub fn at(&self, progress: f32) -> (usize, f32) {
        let progress = progress.clamp(0.0, 1.0);
        let mut start = 0.0;
        let mut index = 0;
        while index < N {
            let end = self.ends[index];
            // A boundary belongs to the later phase (`progress < end`),
            // mirroring the `if progress < 0.1` branching this replaces;
            // the last phase also owns everything at or past its end.
            if progress < end || index + 1 == N {
                let span = end - start;
                // A zero-width phase is already over rather than 0/0.
                let t = if span > 0.0 {
                    (progress - start) / span
                } else {
                    1.0
                };
                return (index, t.clamp(0.0, 1.0));
            }
            start = end;
            index += 1;
        }
        // `N == 0` only: no phase to be in, and no phase to report.
        (0, 1.0)
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

    /// Tolerance for comparing a computed phase-local `t` against a
    /// hand-worked expectation.
    const EPS: f32 = 1e-6;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < EPS,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn phases_report_index_and_progress_within_the_phase() {
        let phases = Phases::new([0.1, 0.4, 0.85, 1.0]);
        assert_eq!(phases.at(0.0), (0, 0.0));
        let (index, t) = phases.at(0.05);
        assert_eq!(index, 0);
        assert_close(t, 0.5);
        let (index, t) = phases.at(0.25);
        assert_eq!(index, 1);
        assert_close(t, 0.5);
        assert_eq!(phases.at(1.0), (3, 1.0));
    }

    #[test]
    fn phase_boundary_belongs_to_the_later_phase() {
        let phases = Phases::new([0.1, 0.4, 0.85, 1.0]);
        assert_eq!(phases.at(0.1), (1, 0.0));
        assert_eq!(phases.at(0.4), (2, 0.0));
        assert_eq!(phases.at(0.85), (3, 0.0));
    }

    #[test]
    fn phases_saturate_outside_zero_to_one() {
        let phases = Phases::new([0.1, 0.4, 0.85, 1.0]);
        assert_eq!(phases.at(-1.0), (0, 0.0));
        assert_eq!(phases.at(2.0), (3, 1.0));
    }

    #[test]
    fn a_single_phase_spans_the_whole_range() {
        let phases = Phases::new([1.0]);
        let (index, t) = phases.at(0.5);
        assert_eq!(index, 0);
        assert_close(t, 0.5);
        assert_eq!(phases.at(0.0), (0, 0.0));
        assert_eq!(phases.at(1.0), (0, 1.0));
    }

    #[test]
    fn a_zero_width_phase_does_not_divide_by_zero() {
        let phases = Phases::new([0.5, 0.5, 1.0]);
        let (index, t) = phases.at(0.5);
        assert_eq!(index, 2);
        assert!(t.is_finite());
        assert_close(t, 0.0);
    }
}
