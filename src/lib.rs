#![warn(missing_docs)]

//! `ttui` — a terminal UI framework built around layered buffers,
//! `Transition`-driven animation, and dumb (no-internal-state) widgets.

/// Terminal event loop and the `App` trait apps implement.
pub mod app;
/// `AudioSink` trait for optional sound-effect playback.
pub mod audio;
/// Alpha-blending prototype — spike prototype, not a committed API.
pub mod blend;
/// Cell/`Buffer`/`LayerStack` — the framework's core render target.
pub mod buffer;
/// Deterministic camera viewport and brightness scaling.
pub mod camera;
/// Sub-cell rendering primitive (half-block + braille) — graduated,
/// committed API; see the module docs for `HalfBlock` vs `Braille`
/// details.
pub mod canvas;
/// Linear/eased interpolation and progress helpers.
pub mod easing;
/// Screen-shake and other whole-buffer visual effects.
pub mod effects;
/// Decaying noise overlay for glitch/corruption effects.
pub mod glitch;
/// Key-binding resolver: single keys and multi-key chords resolving
/// to an app-defined action type.
pub mod input;
/// `Rect`/`Constraint`-based area splitting.
pub mod layout;
/// A simple particle system for bursts and impacts.
pub mod particles;
/// Fixed-forward pinhole-camera projection: points, lines, polygons.
pub mod perspective;
/// Raw-mode terminal setup/teardown.
pub mod terminal;
/// App color palette and border glyph set.
pub mod theme;
/// Time-driven progress tracking for animations, and the `Phases`
/// subdivision of a progress range.
pub mod transition;
/// Ready-to-render widgets — see `widgets` module docs.
pub mod widgets;
