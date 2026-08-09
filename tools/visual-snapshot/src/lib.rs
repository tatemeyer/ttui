//! Headless visual-snapshot tooling for TTUI example apps: spawns a
//! compiled example under a pseudo-console, drives it with a scripted
//! sequence of key presses and waits, and rasterizes the captured
//! terminal output to a PNG or animated GIF.

pub mod color;
pub mod encode;
pub mod glyph;
pub mod keys;
pub mod pty;
pub mod render;
pub mod script;
