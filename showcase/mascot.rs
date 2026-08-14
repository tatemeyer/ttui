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

// `Reacting`/`Grabbing` and their hold durations aren't driven from
// outside this module yet — Task 3 and Task 5 wire up `set_pose`
// calls that switch to them. Only exercised by this module's own
// tests until then.
#[allow(dead_code)]
const REACT_HOLD: Duration = Duration::from_millis(300);
#[allow(dead_code)]
const GRAB_HOLD: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
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
        _ => None,
    }
}

#[rustfmt::skip]
const IDLE: [[u8; 12]; 12] = [
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,0,0,0,0,0,0],
    [0,6,2,2,2,2,2,2,2,6,0,0],
    [0,2,2,4,4,4,4,4,4,2,2,0],
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
    [0,2,2,2,4,4,4,4,2,2,2,0],
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
    [0,2,2,4,4,4,4,4,4,2,2,0],
    [0,2,2,2,2,2,2,2,2,2,2,0],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [6,2,2,2,2,2,2,2,2,2,2,6],
    [0,0,2,2,2,2,2,2,2,2,0,0],
    [0,0,0,1,2,2,2,2,1,0,0,0],
    [0,0,0,0,1,2,2,1,0,0,0,0],
    [0,0,0,0,0,3,3,0,0,0,0,0],
    [0,0,0,0,3,3,3,3,0,0,0,0],
];

pub(crate) struct GripperMascot {
    pose: MascotPose,
    hold: Duration,
}

impl GripperMascot {
    pub(crate) fn new() -> Self {
        GripperMascot {
            pose: MascotPose::Idle,
            hold: Duration::ZERO,
        }
    }

    /// Switches pose immediately. `Reacting`/`Grabbing` auto-settle
    /// back to `Idle` after their hold duration elapses via `tick`.
    #[allow(dead_code)]
    pub(crate) fn set_pose(&mut self, pose: MascotPose) {
        self.pose = pose;
        self.hold = match pose {
            MascotPose::Idle => Duration::ZERO,
            MascotPose::Reacting => REACT_HOLD,
            MascotPose::Grabbing => GRAB_HOLD,
        };
    }

    pub(crate) fn tick(&mut self, elapsed: Duration) {
        if self.hold > Duration::ZERO {
            self.hold = self.hold.saturating_sub(elapsed);
            if self.hold == Duration::ZERO {
                self.pose = MascotPose::Idle;
            }
        }
    }

    /// Draws the current pose's grid, one solid-color `Cell` per
    /// filled pixel, at `area`'s top-left corner. Cells clipped by
    /// `area` (or a grid entry of `0`) are simply skipped.
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer) {
        let grid = match self.pose {
            MascotPose::Idle => &IDLE,
            MascotPose::Reacting => &REACTING,
            MascotPose::Grabbing => &GRABBING,
        };
        for (row, cells) in grid.iter().enumerate() {
            let y = area.y + row as u16;
            if y >= area.y + area.height {
                break;
            }
            for (col, &code) in cells.iter().enumerate() {
                let x = area.x + col as u16;
                if x >= area.x + area.width {
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
}
