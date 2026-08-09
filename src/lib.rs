#![warn(missing_docs)]

//! `ttui` — a terminal UI framework built around layered buffers,
//! `Transition`-driven animation, and dumb (no-internal-state) widgets.

/// Terminal event loop and the `App` trait apps implement.
pub mod app;
/// `AudioSink` trait for optional sound-effect playback.
pub mod audio;
/// Cell/`Buffer`/`LayerStack` — the framework's core render target.
pub mod buffer;
/// Sub-cell rendering primitive (half-block + braille) — spike
/// prototype, not a committed API.
pub mod canvas;
/// Deterministic camera viewport and brightness scaling.
pub mod camera;
/// Linear/eased interpolation and progress helpers.
pub mod easing;
/// Screen-shake and other whole-buffer visual effects.
pub mod effects;
/// Decaying noise overlay for glitch/corruption effects.
pub mod glitch;
/// `Rect`/`Constraint`-based area splitting.
pub mod layout;
/// A simple particle system for bursts and impacts.
pub mod particles;
/// Raw-mode terminal setup/teardown.
pub mod terminal;
/// App color palette and border glyph set.
pub mod theme;
/// Time-driven progress tracking for animations.
pub mod transition;
/// Ready-to-render widgets — see `widgets` module docs.
pub mod widgets;
