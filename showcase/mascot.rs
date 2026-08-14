//! GripperMascot — a 12x12-cell robot rendered as solid-color `Cell`s
//! (bg-fill, the same technique `list.rs`/`block.rs` use for row
//! highlighting), not glyph line-art. Three discrete poses; no
//! tweening between them, matching how every other app in this
//! project holds discrete poses rather than interpolating.

use crossterm::style::Color;
use std::time::Duration;
use ttui::buffer::{Buffer, Cell};
use ttui::layout::Rect;

pub(crate) const MASCOT_WIDTH: u16 = 12;
pub(crate) const MASCOT_HEIGHT: u16 = 12;

const REACT_HOLD: Duration = Duration::from_millis(300);
const GRAB_HOLD: Duration = Duration::from_millis(400);
const BREATHE_INTERVAL: Duration = Duration::from_millis(2000);
const BLINK_INTERVAL: Duration = Duration::from_millis(3500);
const BLINK_DURATION: Duration = Duration::from_millis(150);

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MascotPose {
    Idle,
    Reacting,
    Grabbing,
}

fn palette(code: u8) -> Option<Color> {
    match code {
        1 => Some(Color::Rgb {
            r: 42,
            g: 42,
            b: 42,
        }),
        2 => Some(Color::Rgb {
            r: 138,
            g: 143,
            b: 152,
        }),
        3 => Some(Color::Rgb {
            r: 255,
            g: 140,
            b: 66,
        }),
        4 => Some(Color::Rgb {
            r: 95,
            g: 212,
            b: 255,
        }),
        6 => Some(Color::Rgb {
            r: 199,
            g: 203,
            b: 209,
        }),
        9 => Some(Color::Rgb {
            r: 255,
            g: 255,
            b: 255,
        }),
        _ => None,
    }
}

#[rustfmt::skip]
const IDLE: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,4,9,9,4,4,4,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,3,3,0,3,3,0,0,0,0],
];

#[rustfmt::skip]
const REACTING: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,2,4,9,9,4,2,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,3,3,0,3,3,0,0,0,0],
];

#[rustfmt::skip]
const GRABBING: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,4,9,9,4,4,4,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,0,3,3,3,3,0,0,0,0],
];

#[rustfmt::skip]
const IDLE_B: [[u8; 12]; 12] = [
    [0,0,0,0,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,1,0,1,0,1,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,4,9,9,4,4,4,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,3,3,0,3,3,0,0,0,0],
];

#[rustfmt::skip]
const BLINK: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,1,1,1,1,1,1,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,3,3,0,3,3,0,0,0,0],
];

pub(crate) struct GripperMascot {
    pose: MascotPose,
    hold: Duration,
    breathe_elapsed: Duration,
    blink_elapsed: Duration,
}

impl GripperMascot {
    pub(crate) fn new() -> Self {
        GripperMascot {
            pose: MascotPose::Idle,
            hold: Duration::ZERO,
            breathe_elapsed: Duration::ZERO,
            blink_elapsed: Duration::ZERO,
        }
    }

    /// Switches pose immediately. `Reacting`/`Grabbing` auto-settle
    /// back to `Idle` after their hold duration elapses via `tick`.
    pub(crate) fn set_pose(&mut self, pose: MascotPose) {
        self.pose = pose;
        self.hold = match pose {
            MascotPose::Idle => Duration::ZERO,
            MascotPose::Reacting => REACT_HOLD,
            MascotPose::Grabbing => GRAB_HOLD,
        };
    }

    /// Advances the pose-hold countdown (unchanged) plus two
    /// independent idle-animation timers. Both timers keep
    /// accumulating regardless of the current pose — only `render`
    /// gates their effect to `MascotPose::Idle` — so returning to
    /// `Idle` mid-cycle never causes a stutter or a reset-to-zero jump.
    pub(crate) fn tick(&mut self, elapsed: Duration) {
        if self.hold > Duration::ZERO {
            self.hold = self.hold.saturating_sub(elapsed);
            if self.hold == Duration::ZERO {
                self.pose = MascotPose::Idle;
            }
        }
        self.breathe_elapsed += elapsed;
        while self.breathe_elapsed >= BREATHE_INTERVAL {
            self.breathe_elapsed -= BREATHE_INTERVAL;
        }
        self.blink_elapsed += elapsed;
        let blink_cycle = BLINK_INTERVAL + BLINK_DURATION;
        while self.blink_elapsed >= blink_cycle {
            self.blink_elapsed -= blink_cycle;
        }
    }

    /// Second half of the breathing cycle: antenna/head dip.
    fn is_breathing_b(&self) -> bool {
        self.breathe_elapsed >= BREATHE_INTERVAL / 2
    }

    /// Within the held portion of the blink cycle.
    fn is_blinking(&self) -> bool {
        self.blink_elapsed >= BLINK_INTERVAL
    }

    /// Draws the current pose's grid, one solid-color `Cell` per
    /// filled pixel, at `area`'s top-left corner. Cells clipped by
    /// `area` (or a grid entry of `0`) are simply skipped.
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer) {
        let grid = match self.pose {
            MascotPose::Idle => {
                if self.is_blinking() {
                    &BLINK
                } else if self.is_breathing_b() {
                    &IDLE_B
                } else {
                    &IDLE
                }
            }
            MascotPose::Reacting => &REACTING,
            MascotPose::Grabbing => &GRABBING,
        };
        for (row, cells) in grid.iter().enumerate() {
            let y = area.y + row as u16;
            // Clipped against the buffer's actual bounds, not
            // `area.height`/`area.width` — `area` here is always the
            // mascot's own 12x12 rect, so `row`/`col` (both < 12)
            // could never trip a check against it. Without this, a
            // vignette that positions the mascot near the bottom/right
            // edge of a small terminal (e.g. Assembly Line's
            // reach-down, which needs `MASCOT_Y_OFFSET +
            // REACH_DOWN_OFFSET + MASCOT_HEIGHT` rows) would panic in
            // `Buffer::set` the moment it tried to draw past the real
            // buffer.
            if y >= buf.height {
                break;
            }
            for (col, &code) in cells.iter().enumerate() {
                let x = area.x + col as u16;
                if x >= buf.width {
                    break;
                }
                if let Some(color) = palette(code) {
                    buf.set(
                        x,
                        y,
                        Cell {
                            symbol: ' ',
                            fg: Color::Reset,
                            bg: color,
                            alpha: 1.0,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_mascot_starts_idle() {
        let m = GripperMascot::new();
        assert!(m.pose == MascotPose::Idle);
    }

    #[test]
    fn reacting_settles_back_to_idle_after_its_hold_elapses() {
        let mut m = GripperMascot::new();
        m.set_pose(MascotPose::Reacting);
        m.tick(REACT_HOLD);
        assert!(m.pose == MascotPose::Idle);
    }

    #[test]
    fn reacting_stays_active_before_its_hold_elapses() {
        let mut m = GripperMascot::new();
        m.set_pose(MascotPose::Reacting);
        m.tick(REACT_HOLD - Duration::from_millis(1));
        assert!(m.pose == MascotPose::Reacting);
    }

    #[test]
    fn render_skips_transparent_cells() {
        let m = GripperMascot::new();
        let mut buf = Buffer::new(MASCOT_WIDTH, MASCOT_HEIGHT);
        m.render(
            Rect {
                x: 0,
                y: 0,
                width: MASCOT_WIDTH,
                height: MASCOT_HEIGHT,
            },
            &mut buf,
        );
        // Grid row 0, col 0 is a `0` (transparent) in every pose.
        assert_eq!(*buf.get(0, 0), Cell::default());
        // Grid row 2, col 2 is a `2` (body) in every pose.
        assert_ne!(*buf.get(2, 2), Cell::default());
    }

    #[test]
    fn breathing_toggles_to_b_after_half_the_interval() {
        let mut m = GripperMascot::new();
        m.tick(BREATHE_INTERVAL / 2);
        assert!(m.is_breathing_b());
    }

    #[test]
    fn breathing_stays_a_before_half_the_interval() {
        let mut m = GripperMascot::new();
        m.tick(BREATHE_INTERVAL / 2 - Duration::from_millis(1));
        assert!(!m.is_breathing_b());
    }

    #[test]
    fn breathing_wraps_back_to_a_after_a_full_interval() {
        let mut m = GripperMascot::new();
        m.tick(BREATHE_INTERVAL);
        assert!(!m.is_breathing_b());
    }

    #[test]
    fn blinking_starts_after_the_blink_interval() {
        let mut m = GripperMascot::new();
        m.tick(BLINK_INTERVAL);
        assert!(m.is_blinking());
    }

    #[test]
    fn blinking_stays_false_before_the_blink_interval() {
        let mut m = GripperMascot::new();
        m.tick(BLINK_INTERVAL - Duration::from_millis(1));
        assert!(!m.is_blinking());
    }

    #[test]
    fn blinking_ends_after_its_own_duration_and_wraps() {
        let mut m = GripperMascot::new();
        m.tick(BLINK_INTERVAL + BLINK_DURATION);
        assert!(!m.is_blinking());
    }

    #[test]
    fn idle_timers_keep_accumulating_while_reacting() {
        let mut m = GripperMascot::new();
        m.set_pose(MascotPose::Reacting);
        m.tick(BREATHE_INTERVAL / 2);
        // Pose is still Reacting (REACT_HOLD is 300ms, well under
        // BREATHE_INTERVAL/2's 1000ms), so breathing has no visible
        // effect yet, but the timer itself must have kept moving —
        // verified indirectly: once it settles back to Idle, the
        // breathing phase should already reflect the elapsed time.
        m.tick(REACT_HOLD); // settles back to Idle (300ms < what's left of the hold)
        assert!(m.is_breathing_b());
    }

    #[test]
    fn render_selects_blink_grid_during_the_blink_window() {
        let mut m = GripperMascot::new();
        m.tick(BLINK_INTERVAL);
        let mut buf = Buffer::new(MASCOT_WIDTH, MASCOT_HEIGHT);
        m.render(
            Rect {
                x: 0,
                y: 0,
                width: MASCOT_WIDTH,
                height: MASCOT_HEIGHT,
            },
            &mut buf,
        );
        // BLINK's row 3 is all code-1 (trim) across cols 3-8 — no
        // code-4/9 (visor) cells should be present on that row.
        for x in 3..9 {
            assert_ne!(
                buf.get(x, 3).bg,
                Color::Rgb {
                    r: 95,
                    g: 212,
                    b: 255
                },
                "visor should be dark during a blink, col {x}"
            );
        }
    }
}
