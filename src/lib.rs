#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/ttui/1.1.0")]

//! `ttui` — a terminal UI framework built around layered buffers,
//! `Transition`-driven animation, and dumb (no-internal-state) widgets.
//!
//! # Quick start
//!
//! An app is one type implementing [`App`](app::App). You own the state;
//! `ttui` owns the loop, the diffing, and the terminal.
//!
//! ```no_run
//! use crossterm::event::{Event, KeyCode};
//! use ttui::app::{run, App};
//! use ttui::buffer::LayerStack;
//! use ttui::layout::Rect;
//! use ttui::widgets::text::Text;
//!
//! struct Hello {
//!     quit: bool,
//! }
//!
//! impl App for Hello {
//!     fn update(&mut self, event: &Event) {
//!         if let Event::Key(key) = event {
//!             if key.code == KeyCode::Char('q') {
//!                 self.quit = true;
//!             }
//!         }
//!     }
//!
//!     fn view(&self, area: Rect, buf: &mut LayerStack) {
//!         Text::new("hello, terminal — press q to quit").render(area, buf);
//!     }
//!
//!     fn should_quit(&self) -> bool {
//!         self.quit
//!     }
//! }
//!
//! fn main() -> std::io::Result<()> {
//!     run(&mut Hello { quit: false })
//! }
//! ```
//!
//! [`run`](app::run) enables raw mode, installs a panic hook that restores
//! the terminal, and only redraws cells that actually changed.
//!
//! # How it fits together
//!
//! ```text
//! App state ──view()──▶ LayerStack ──composite()──▶ Buffer ──diff()──▶ terminal
//!     ▲                                                                    │
//!     └──────────────── update(event) / on_tick(elapsed) ◀─────────────────┘
//! ```
//!
//! - **[`buffer`]** — [`Cell`](buffer::Cell), [`Buffer`](buffer::Buffer) and
//!   [`LayerStack`](buffer::LayerStack), the render target. Layers composite
//!   top-to-bottom by alpha, so an effects layer can sit over a UI layer
//!   without either knowing about the other.
//! - **[`layout`]** — splits a [`Rect`](layout::Rect) by
//!   [`Constraint`](layout::Constraint)s. No widget computes its own position.
//! - **[`widgets`]** — stateless renderers. A widget is handed an area and a
//!   buffer and draws; it never holds selection or scroll state, so your app
//!   stays the single source of truth.
//! - **[`transition`]** — [`Transition`](transition::Transition) tracks
//!   real elapsed time as `0.0..=1.0` progress, and
//!   [`Phases`](transition::Phases) subdivides that progress into named
//!   stages without writing every boundary twice.
//! - **[`theme`]** — one palette and border set threaded through widgets.
//!
//! Beyond the core: [`canvas`] (sub-cell half-block and braille drawing),
//! [`perspective`] (fixed-forward 3D projection), [`particles`], [`glitch`],
//! [`effects`], [`camera`], [`input`] (key chords), [`easing`], [`noise`] and
//! [`audio`].
//!
//! # Animation
//!
//! Return a [`tick_rate`](app::App::tick_rate) and `ttui` calls
//! [`on_tick`](app::App::on_tick) with the *real* elapsed time, so animation
//! stays correct when a frame runs long:
//!
//! ```
//! use std::time::Duration;
//! use ttui::transition::{Phases, Transition};
//!
//! // Three stages across one 900ms transition: 20% fade in, 60% hold,
//! // 20% fade out. Boundaries are written once, not twice.
//! const STAGES: Phases<3> = Phases::new([0.2, 0.8, 1.0]);
//!
//! let t = Transition::start(Duration::from_millis(900));
//! let (stage, progress_within) = STAGES.at(t.progress());
//! assert!(stage < 3);
//! assert!((0.0..=1.0).contains(&progress_within));
//! ```
//!
//! # Examples
//!
//! The repository carries ten runnable example apps and a `showcase` demo
//! reel. From a checkout:
//!
//! ```text
//! cargo run --bin showcase        # mascot-hosted reel of five vignettes
//! cargo run --example demo        # nested panes, Tab focus, list + table
//! cargo run --example falcon      # cockpit: starfield, HUD, glitch effects
//! ```
//!
//! See [`examples/README.md`] for what each one demonstrates.
//!
//! [`examples/README.md`]: https://github.com/tatemeyer/ttui/blob/main/examples/README.md

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
/// Deterministic seed-driven jitter for scattering and placement.
pub mod noise;
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
