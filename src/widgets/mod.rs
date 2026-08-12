//! Ready-to-render TUI widgets — each takes an explicit value/theme
//! and a target area, no internal animation state; the owning app
//! computes any tweened value and passes a snapshot per frame.

/// Two-position analog toggle switch.
pub mod analog_toggle;
/// Horizontal bar chart with labeled, max-scaled bars.
pub mod bar_chart;
/// Bordered container, with an optional outward second border ring.
pub mod block;
/// Thick, riveted, deliberately-asymmetric double-line border.
pub mod cockpit_panel;
/// Percent display that shifts white/yellow/red as it climbs.
pub mod damage_meter;
/// Circular item-select dial.
pub mod dial;
/// Alternating two-color text row with a trailing cursor glyph.
pub mod dna_console;
/// Segmented circular progress ring.
pub mod energy_core;
/// Scrollable selectable list.
pub mod list;
/// Pulsing circular decoration glyph.
pub mod roundel;
/// Jerky, two-frame navigation cursor.
pub mod scuttle_cursor;
/// Three-ring beveled border, drawn inward.
pub mod smash_border;
/// Header-row-plus-data-rows table.
pub mod table;
/// Single-line plain text.
pub mod text;
/// Braille-glyph rotating speed indicator.
pub mod time_rotor;
