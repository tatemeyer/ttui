//! Test-only fixtures shared across widget test modules (`selection`,
//! `list`, `dial`, `table`). Exists because a `#[cfg(test)] mod tests`
//! nested in another widget's file is private to that file — a sibling
//! module's tests cannot reach it, so the fixture needs its own home.

use crate::buffer::CellStyle;
use crate::theme::{BorderSet, Theme};
use crossterm::style::Color;

/// A fully-populated `Theme` fixture. `Theme` has no `Default`, so
/// every field must be set explicitly here.
pub(crate) fn test_theme() -> Theme {
    Theme {
        background: Color::Rgb { r: 0, g: 0, b: 32 },
        primary: Color::Rgb { r: 0, g: 255, b: 0 },
        secondary: Color::Rgb {
            r: 0,
            g: 128,
            b: 255,
        },
        tertiary: Color::Rgb {
            r: 255,
            g: 255,
            b: 0,
        },
        accent: Color::Rgb { r: 255, g: 0, b: 0 },
        primary_end: None,
        border: BorderSet::single_line(),
        border_style: CellStyle::default(),
        border_thick: false,
    }
}
