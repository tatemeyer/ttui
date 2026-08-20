//! `#[non_exhaustive]` only binds outside the defining crate, so this
//! lives in `tests/` rather than an inline module.

use ttui::buffer::Intensity;
use ttui::canvas::CanvasMode;
use ttui::layout::{Constraint, Direction};

// A wildcard arm is REQUIRED for each of these to compile from another
// crate. If someone removes `#[non_exhaustive]`, these still compile —
// so each is paired with a construction check below to prove the enum
// is still usable, and the real guard is the doc comment on each enum.
#[test]
fn non_exhaustive_enums_still_match_with_a_wildcard_arm() {
    let i = Intensity::Bold;
    let described = match i {
        Intensity::Normal => "normal",
        Intensity::Bold => "bold",
        Intensity::Dim => "dim",
        _ => "unknown",
    };
    assert_eq!(described, "bold");

    let m = CanvasMode::Braille;
    let described = match m {
        CanvasMode::HalfBlock => "half",
        CanvasMode::Braille => "braille",
        _ => "unknown",
    };
    assert_eq!(described, "braille");

    let d = Direction::Horizontal;
    let described = match d {
        Direction::Horizontal => "h",
        Direction::Vertical => "v",
        _ => "unknown",
    };
    assert_eq!(described, "h");

    let c = Constraint::Fill(1);
    let described = match c {
        Constraint::Fixed(_) => "fixed",
        Constraint::Percentage(_) => "pct",
        Constraint::Min(_) => "min",
        Constraint::Fill(_) => "fill",
        _ => "unknown",
    };
    assert_eq!(described, "fill");
}

#[test]
fn non_exhaustive_enums_are_still_constructible_from_another_crate() {
    // `#[non_exhaustive]` on an enum restricts exhaustive matching, not
    // variant construction. This test exists so a future reader does
    // not "fix" that by reaching for `#[non_exhaustive]` on variants,
    // which WOULD block construction and break every caller.
    let _ = Intensity::Dim;
    let _ = CanvasMode::HalfBlock;
    let _ = Direction::Vertical;
    let _ = Constraint::Percentage(50);
}
