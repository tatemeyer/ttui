# Visual Snapshot Tooling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `tools/visual-snapshot`, a standalone crate that spawns a
compiled TTUI example under a real pseudo-console (`portable-pty`),
drives it with scripted key input and real-time waits, captures its
terminal output via `vt100`, and rasterizes the result to a PNG (single
frame) or animated GIF (multiple frames) an agent can `Read` directly —
closing the "no eyes on the terminal" gap described in
`docs/tooling/visual-review.md` and
`docs/design/specs/core/2026-08-09-visual-snapshot-tooling-design.md`.

**Architecture:** A new workspace member with no dependency on `ttui`
itself — it talks to example binaries purely as subprocesses over PTY
bytes, using `vt100` to turn the raw ANSI stream into structured cell
state and `font8x8` + `image` to rasterize that state to pixels. Six
independent, unit-testable pure-logic modules (script parsing, key
encoding, color mapping, glyph lookup, rendering, file encoding) sit
behind one orchestration module (`pty.rs`) that owns the only
non-deterministic, subprocess-dependent code, and a thin CLI (`main.rs`)
wires them together.

**Tech Stack:** Rust, `portable-pty` (PTY spawning), `vt100` (ANSI
stream parsing), `font8x8` (bitmap glyphs), `image` (PNG/GIF encoding),
`serde`/`serde_json` (script parsing), `clap` (CLI arg parsing).

## Global Constraints

- The `ttui` library's own `Cargo.toml` (`[dependencies]` and
  `[dev-dependencies]`) is **not modified** by this plan — every new
  dependency lives in `tools/visual-snapshot/Cargo.toml`.
- Root `Cargo.toml` gains exactly one addition: a `[workspace]` table
  with `members = [".", "tools/visual-snapshot"]`.
- TDD applies to every module in `tools/visual-snapshot/src/` except
  Task 1 (pure scaffolding/config, no logic) and Task 11 (docs-only) —
  both TDD exceptions already named in
  `.claude/rules/development-conventions.md`.
- `vt100::Cell` exposes `bold()`, `italic()`, `underline()`, `inverse()`
  only — no `dim()`/`strikethrough()`. The rasterizer renders bold
  (brighten), inverse (swap fg/bg), and underline (line overlay);
  italic and dim/strikethrough are not visually rendered — confirmed
  API constraint, not a task to work around.
- A glyph outside `font8x8`'s coverage is a **hard error** naming the
  missing codepoint — no image is written. `EnergyCore`'s `✦`
  (U+2726) is a known, expected trigger of this path (confirmed:
  `font8x8`'s `MISC_FONTS` doesn't reach the Dingbats block).
- Commit after every task, per `.claude/rules/development-conventions.md`
  (Conventional Commits, `scope: tooling`, one commit per task).

---

## File Structure

```
Cargo.toml                              (root — gains [workspace])
tools/visual-snapshot/
  Cargo.toml
  src/
    lib.rs           — re-exports every module for integration tests
    main.rs           — CLI entry point
    script.rs         — Step enum + JSON script parsing
    keys.rs            — named key -> raw byte sequence encoder
    color.rs           — vt100::Color -> image::Rgb<u8>, brighten/swap helpers
    glyph.rs           — font8x8 glyph lookup, hard-errors on unmapped chars
    render.rs           — vt100::Screen -> RgbaImage (2x upscaled, styled)
    encode.rs            — RgbaImage(s) -> PNG / animated GIF file
    pty.rs                — spawn/drive/capture: the only subprocess-dependent module
  examples/
    echo_key.rs          — fixture binary: reads keys via crossterm, echoes their Debug repr
  tests/
    pty_roundtrip.rs      — integration tests for pty.rs against echo_key
```

---

### Task 1: Workspace scaffolding

**Files:**
- Modify: `Cargo.toml:1-16` (root)
- Create: `tools/visual-snapshot/Cargo.toml`
- Create: `tools/visual-snapshot/src/lib.rs`
- Create: `tools/visual-snapshot/src/main.rs`

**Interfaces:**
- Produces: a buildable, empty `visual-snapshot` crate later tasks add modules to.

**Tag: pure config, no application logic — TDD exception per `development-conventions.md`.**

- [ ] **Step 1: Add the `[workspace]` table to root `Cargo.toml`**

Add after the `[package]` block in `Cargo.toml`:

```toml
[workspace]
members = [".", "tools/visual-snapshot"]
```

The root package itself must also appear as a workspace member (`"."`) — Cargo doesn't imply this automatically once a `[workspace]` table exists.

- [ ] **Step 2: Create `tools/visual-snapshot/Cargo.toml`**

```toml
[package]
name = "visual-snapshot"
version = "0.1.0"
edition = "2021"

[dependencies]
portable-pty = "0.9"
vt100 = "0.15"
image = "0.25"
font8x8 = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }

[dev-dependencies]
crossterm = "0.27"
tempfile = "3"

[[example]]
name = "echo_key"
```

`crossterm` here is scoped to `visual-snapshot`'s own dev-dependencies for its test fixture (Task 8) — it never touches the `ttui` library's `Cargo.toml`. Version pins match what's already used elsewhere in this repo (`crossterm = "0.27"` in the root `Cargo.toml`) where a shared crate is involved.

- [ ] **Step 3: Create a minimal `src/lib.rs`**

```rust
//! Headless visual-snapshot tooling for TTUI example apps: spawns a
//! compiled example under a pseudo-console, drives it with a scripted
//! sequence of key presses and waits, and rasterizes the captured
//! terminal output to a PNG or animated GIF.
```

(Module declarations are added by each later task as its module lands — an empty crate-doc comment is the whole deliverable here.)

- [ ] **Step 4: Create a minimal `src/main.rs`**

```rust
fn main() {
    println!("visual-snapshot: not yet implemented");
}
```

- [ ] **Step 5: Verify the workspace builds**

Run: `cargo build --workspace`
Expected: both `ttui` and `visual-snapshot` compile with no errors. Running `cargo run -p visual-snapshot` prints the placeholder line.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml tools/visual-snapshot/Cargo.toml tools/visual-snapshot/src/lib.rs tools/visual-snapshot/src/main.rs
git commit -m "chore(tooling): scaffold visual-snapshot workspace crate"
```

---

### Task 2: Key encoder

**Files:**
- Create: `tools/visual-snapshot/src/keys.rs`
- Modify: `tools/visual-snapshot/src/lib.rs` (add `pub mod keys;`)

**Interfaces:**
- Produces: `pub fn encode_key(name: &str) -> Result<Vec<u8>, KeyEncodeError>` and `pub enum KeyEncodeError { Unknown(String) }` — consumed by `pty.rs` (Task 9) to turn a script's `{"key": "..."}` step into raw bytes written to the PTY's input handle.

- [ ] **Step 1: Write the failing tests**

```rust
// tools/visual-snapshot/src/keys.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_keys_encode_as_csi_sequences() {
        assert_eq!(encode_key("Right").unwrap(), b"\x1b[C".to_vec());
        assert_eq!(encode_key("Left").unwrap(), b"\x1b[D".to_vec());
        assert_eq!(encode_key("Up").unwrap(), b"\x1b[A".to_vec());
        assert_eq!(encode_key("Down").unwrap(), b"\x1b[B".to_vec());
    }

    #[test]
    fn enter_esc_tab_encode_correctly() {
        assert_eq!(encode_key("Enter").unwrap(), b"\r".to_vec());
        assert_eq!(encode_key("Esc").unwrap(), b"\x1b".to_vec());
        assert_eq!(encode_key("Tab").unwrap(), b"\t".to_vec());
    }

    #[test]
    fn single_char_keys_encode_as_their_own_byte() {
        assert_eq!(encode_key("a").unwrap(), b"a".to_vec());
        assert_eq!(encode_key("Q").unwrap(), b"Q".to_vec());
        assert_eq!(encode_key("5").unwrap(), b"5".to_vec());
    }

    #[test]
    fn ctrl_combos_encode_as_control_bytes() {
        // Ctrl+A..Ctrl+Z map to bytes 0x01..0x1a
        assert_eq!(encode_key("Ctrl+A").unwrap(), vec![0x01]);
        assert_eq!(encode_key("Ctrl+C").unwrap(), vec![0x03]);
        assert_eq!(encode_key("Ctrl+Z").unwrap(), vec![0x1a]);
    }

    #[test]
    fn unknown_key_name_is_an_error() {
        let err = encode_key("Nonsense").unwrap_err();
        assert_eq!(err, KeyEncodeError::Unknown("Nonsense".to_string()));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p visual-snapshot keys::`
Expected: FAIL — `encode_key`/`KeyEncodeError` not defined.

- [ ] **Step 3: Implement `encode_key`**

```rust
// tools/visual-snapshot/src/keys.rs (above the tests module)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEncodeError {
    Unknown(String),
}

/// Encodes a named key (as used in a snapshot script's `{"key": "..."}`
/// step) into the raw byte sequence a real terminal would send for it.
pub fn encode_key(name: &str) -> Result<Vec<u8>, KeyEncodeError> {
    match name {
        "Up" => Ok(b"\x1b[A".to_vec()),
        "Down" => Ok(b"\x1b[B".to_vec()),
        "Right" => Ok(b"\x1b[C".to_vec()),
        "Left" => Ok(b"\x1b[D".to_vec()),
        "Enter" => Ok(b"\r".to_vec()),
        "Esc" => Ok(b"\x1b".to_vec()),
        "Tab" => Ok(b"\t".to_vec()),
        _ => {
            if let Some(letter) = name.strip_prefix("Ctrl+") {
                let mut chars = letter.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else {
                    return Err(KeyEncodeError::Unknown(name.to_string()));
                };
                let upper = c.to_ascii_uppercase();
                if upper.is_ascii_alphabetic() {
                    return Ok(vec![(upper as u8) - b'A' + 1]);
                }
                return Err(KeyEncodeError::Unknown(name.to_string()));
            }
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii() => Ok(vec![c as u8]),
                _ => Err(KeyEncodeError::Unknown(name.to_string())),
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p visual-snapshot keys::`
Expected: PASS (5 tests).

- [ ] **Step 5: Wire the module in and commit**

Add `pub mod keys;` to `tools/visual-snapshot/src/lib.rs`.

```bash
git add tools/visual-snapshot/src/keys.rs tools/visual-snapshot/src/lib.rs
git commit -m "feat(tooling): add snapshot script key-name encoder"
```

---

### Task 3: Color mapping

**Files:**
- Create: `tools/visual-snapshot/src/color.rs`
- Modify: `tools/visual-snapshot/src/lib.rs` (add `pub mod color;`)

**Interfaces:**
- Consumes: `vt100::Color` (`Default`, `Idx(u8)`, `Rgb(u8, u8, u8)` — confirmed via docs.rs).
- Produces: `pub fn to_rgb(c: vt100::Color) -> image::Rgb<u8>`, `pub fn brighten(c: image::Rgb<u8>) -> image::Rgb<u8>`, `pub fn swap(fg: image::Rgb<u8>, bg: image::Rgb<u8>) -> (image::Rgb<u8>, image::Rgb<u8>)` — consumed by `render.rs` (Task 5).

- [ ] **Step 1: Write the failing tests**

```rust
// tools/visual-snapshot/src/color.rs
#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;
    use vt100::Color;

    #[test]
    fn default_color_maps_to_the_conventional_terminal_default() {
        // Matches the ANSI convention: default fg is light gray, default bg is black.
        assert_eq!(to_rgb(Color::Default), Rgb([229, 229, 229]));
    }

    #[test]
    fn rgb_color_passes_through_unchanged() {
        assert_eq!(to_rgb(Color::Rgb(10, 20, 30)), Rgb([10, 20, 30]));
    }

    #[test]
    fn basic_16_indices_map_to_standard_ansi_colors() {
        assert_eq!(to_rgb(Color::Idx(1)), Rgb([205, 0, 0])); // red
        assert_eq!(to_rgb(Color::Idx(2)), Rgb([0, 205, 0])); // green
        assert_eq!(to_rgb(Color::Idx(9)), Rgb([255, 0, 0])); // bright red
    }

    #[test]
    fn color_cube_indices_map_via_the_standard_xterm_formula() {
        // idx 16 is the cube's origin (0,0,0) -> black.
        assert_eq!(to_rgb(Color::Idx(16)), Rgb([0, 0, 0]));
        // idx 231 is the cube's far corner (5,5,5) -> white-ish (255,255,255).
        assert_eq!(to_rgb(Color::Idx(231)), Rgb([255, 255, 255]));
    }

    #[test]
    fn grayscale_ramp_indices_map_to_increasing_gray() {
        assert_eq!(to_rgb(Color::Idx(232)), Rgb([8, 8, 8]));
        assert_eq!(to_rgb(Color::Idx(255)), Rgb([238, 238, 238]));
    }

    #[test]
    fn brighten_lightens_each_channel_toward_white_and_saturates() {
        assert_eq!(brighten(Rgb([100, 100, 100])), Rgb([140, 140, 140]));
        assert_eq!(brighten(Rgb([250, 0, 0])), Rgb([255, 40, 40]));
    }

    #[test]
    fn swap_exchanges_fg_and_bg() {
        let fg = Rgb([1, 2, 3]);
        let bg = Rgb([4, 5, 6]);
        assert_eq!(swap(fg, bg), (bg, fg));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p visual-snapshot color::`
Expected: FAIL — module contents not defined.

- [ ] **Step 3: Implement the color module**

```rust
// tools/visual-snapshot/src/color.rs (above the tests module)
use image::Rgb;
use vt100::Color;

const BASIC_16: [Rgb<u8>; 16] = [
    Rgb([0, 0, 0]),       // 0 black
    Rgb([205, 0, 0]),     // 1 red
    Rgb([0, 205, 0]),     // 2 green
    Rgb([205, 205, 0]),   // 3 yellow
    Rgb([0, 0, 238]),     // 4 blue
    Rgb([205, 0, 205]),   // 5 magenta
    Rgb([0, 205, 205]),   // 6 cyan
    Rgb([229, 229, 229]), // 7 white
    Rgb([127, 127, 127]), // 8 bright black
    Rgb([255, 0, 0]),     // 9 bright red
    Rgb([0, 255, 0]),     // 10 bright green
    Rgb([255, 255, 0]),   // 11 bright yellow
    Rgb([92, 92, 255]),   // 12 bright blue
    Rgb([255, 0, 255]),   // 13 bright magenta
    Rgb([0, 255, 255]),   // 14 bright cyan
    Rgb([255, 255, 255]), // 15 bright white
];

fn ansi256_to_rgb(idx: u8) -> Rgb<u8> {
    match idx {
        0..=15 => BASIC_16[idx as usize],
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i % 36) / 6;
            let b = i % 6;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            Rgb([level(r), level(g), level(b)])
        }
        232..=255 => {
            let v = 8 + (idx - 232) * 10;
            Rgb([v, v, v])
        }
    }
}

/// Maps a parsed `vt100::Color` to an RGB pixel value.
pub fn to_rgb(c: Color) -> Rgb<u8> {
    match c {
        Color::Default => Rgb([229, 229, 229]),
        Color::Idx(i) => ansi256_to_rgb(i),
        Color::Rgb(r, g, b) => Rgb([r, g, b]),
    }
}

/// Approximates SGR bold by lightening each channel toward white.
pub fn brighten(c: Rgb<u8>) -> Rgb<u8> {
    Rgb([
        c.0[0].saturating_add(40),
        c.0[1].saturating_add(40),
        c.0[2].saturating_add(40),
    ])
}

/// Approximates SGR reverse video by exchanging fg/bg.
pub fn swap(fg: Rgb<u8>, bg: Rgb<u8>) -> (Rgb<u8>, Rgb<u8>) {
    (bg, fg)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p visual-snapshot color::`
Expected: PASS (7 tests). If `vt100::Color`'s variant names or field shapes differ from what's assumed here, the compiler error will name the actual shape — adjust `to_rgb`'s `match` arms to match (this was confirmed against docs.rs during spec writing, but confirm against the actual pulled dependency's version too since `vt100 = "0.15"` may resolve to a patch release with a differently-documented-but-compatible shape).

- [ ] **Step 5: Wire the module in and commit**

Add `pub mod color;` to `tools/visual-snapshot/src/lib.rs`.

```bash
git add tools/visual-snapshot/src/color.rs tools/visual-snapshot/src/lib.rs
git commit -m "feat(tooling): add vt100 Color to RGB mapping"
```

---

### Task 4: Glyph rasterizer

**Files:**
- Create: `tools/visual-snapshot/src/glyph.rs`
- Modify: `tools/visual-snapshot/src/lib.rs` (add `pub mod glyph;`)

**Interfaces:**
- Produces: `pub fn glyph_for(ch: char) -> Result<[u8; 8], GlyphError>`, `pub enum GlyphError { Unmapped(char) }` — consumed by `render.rs` (Task 5).

- [ ] **Step 1: Write the failing tests**

```rust
// tools/visual-snapshot/src/glyph.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_letter_resolves_to_a_nonblank_bitmap() {
        let bitmap = glyph_for('A').unwrap();
        assert!(bitmap.iter().any(|row| *row != 0), "expected 'A' to draw at least one pixel");
    }

    #[test]
    fn space_and_a_resolve_to_different_bitmaps() {
        assert_ne!(glyph_for(' ').unwrap(), glyph_for('A').unwrap());
    }

    #[test]
    fn block_element_glyphs_resolve() {
        // The block-elements TTUI widgets actually emit (src/glitch.rs, src/canvas.rs).
        for ch in ['░', '▒', '▓', '█', '▀', '▄', '▌'] {
            glyph_for(ch).unwrap_or_else(|e| panic!("expected {ch:?} to be mapped, got {e:?}"));
        }
    }

    #[test]
    fn ascii_border_glyphs_resolve() {
        for ch in ['-', '|', '+'] {
            glyph_for(ch).unwrap();
        }
    }

    #[test]
    fn dingbat_star_is_unmapped() {
        // Confirmed during spec review: font8x8's MISC_FONTS does not
        // reach the Dingbats block. EnergyCore's charged-state glyph
        // is expected to hit this path in real use.
        let err = glyph_for('\u{2726}').unwrap_err();
        assert_eq!(err, GlyphError::Unmapped('\u{2726}'));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p visual-snapshot glyph::`
Expected: FAIL — `glyph_for`/`GlyphError` not defined.

- [ ] **Step 3: Implement the glyph module**

First confirm `font8x8`'s exact lookup API for the pulled dependency version: run `cargo doc -p font8x8 --open` (or read `~/.cargo/registry/src/*/font8x8-*/src/lib.rs`) and note the real signature of the `UnicodeFonts::get` method and the exported table constants (expected shape, per docs.rs: `BASIC_FONTS`, `LATIN_FONTS`, `BLOCK_FONTS`, `BOX_FONTS`, `MISC_FONTS` each implementing a trait with a `get(char) -> Option<[u8; 8]>`-shaped method). Adjust the code below to match whatever the compiler/docs confirm.

```rust
// tools/visual-snapshot/src/glyph.rs (above the tests module)
use font8x8::{UnicodeFonts, BASIC_FONTS, BLOCK_FONTS, BOX_FONTS, LATIN_FONTS, MISC_FONTS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphError {
    Unmapped(char),
}

/// Looks up `ch`'s 8x8 bitmap across every font8x8 table TTUI's actual
/// glyph set draws from. Returns a hard error naming the codepoint if
/// none of them cover it — see `GlyphError::Unmapped`.
pub fn glyph_for(ch: char) -> Result<[u8; 8], GlyphError> {
    BASIC_FONTS
        .get(ch)
        .or_else(|| LATIN_FONTS.get(ch))
        .or_else(|| BLOCK_FONTS.get(ch))
        .or_else(|| BOX_FONTS.get(ch))
        .or_else(|| MISC_FONTS.get(ch))
        .ok_or(GlyphError::Unmapped(ch))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p visual-snapshot glyph::`
Expected: PASS (5 tests).

- [ ] **Step 5: Wire the module in and commit**

Add `pub mod glyph;` to `tools/visual-snapshot/src/lib.rs`.

```bash
git add tools/visual-snapshot/src/glyph.rs tools/visual-snapshot/src/lib.rs
git commit -m "feat(tooling): add font8x8 glyph lookup with hard-error on unmapped chars"
```

---

### Task 5: Render pipeline

**Files:**
- Create: `tools/visual-snapshot/src/render.rs`
- Modify: `tools/visual-snapshot/src/lib.rs` (add `pub mod render;`)

**Interfaces:**
- Consumes: `color::to_rgb`, `color::brighten`, `color::swap` (Task 3); `glyph::glyph_for`, `glyph::GlyphError` (Task 4); `vt100::Screen`.
- Produces: `pub fn render_screen(screen: &vt100::Screen) -> Result<image::RgbaImage, RenderError>`, `pub enum RenderError { Glyph(glyph::GlyphError, u16, u16) }` — consumed by `pty.rs` (Task 9).

- [ ] **Step 1: Write the failing tests**

```rust
// tools/visual-snapshot/src/render.rs
#[cfg(test)]
mod tests {
    use super::*;

    const CELL_PX: u32 = 16; // 8x8 glyph, 2x upscaled

    fn parse(bytes: &[u8]) -> vt100::Parser {
        let mut p = vt100::Parser::new(1, 4, 0);
        p.process(bytes);
        p
    }

    #[test]
    fn image_dimensions_match_screen_size_times_cell_px() {
        let parser = parse(b"abcd");
        let img = render_screen(parser.screen()).unwrap();
        assert_eq!(img.width(), 4 * CELL_PX);
        assert_eq!(img.height(), 1 * CELL_PX);
    }

    #[test]
    fn plain_text_uses_default_fg_over_default_bg() {
        let parser = parse(b"a");
        let img = render_screen(parser.screen()).unwrap();
        // A glyph's background rectangle should show through wherever
        // the 8x8 'a' bitmap has no set pixel — check a corner pixel
        // known to be background for every font8x8 letterform.
        let bg_pixel = img.get_pixel(0, 0);
        assert_eq!(*bg_pixel, image::Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn bold_text_brightens_the_foreground() {
        let parser = parse(b"\x1b[1ma\x1b[0m");
        let img = render_screen(parser.screen()).unwrap();
        // Find a foreground-colored pixel (non-background) and confirm
        // it's brighter than the plain-text case's foreground would be.
        let has_bright_pixel = img.pixels().any(|p| p.0[0] > 229);
        assert!(has_bright_pixel, "expected a brightened foreground pixel");
    }

    #[test]
    fn reverse_video_swaps_fg_and_bg_across_the_whole_cell() {
        let parser = parse(b"\x1b[7ma\x1b[0m");
        let img = render_screen(parser.screen()).unwrap();
        // With fg/bg swapped, the corner background pixel becomes the
        // (light) default foreground color instead of black.
        let corner = img.get_pixel(0, 0);
        assert_eq!(*corner, image::Rgba([229, 229, 229, 255]));
    }

    #[test]
    fn underline_draws_a_line_on_the_bottom_row_of_the_cell() {
        let parser = parse(b"\x1b[4m \x1b[0m"); // underlined space
        let img = render_screen(parser.screen()).unwrap();
        let bottom_row_pixel = img.get_pixel(0, CELL_PX - 1);
        assert_eq!(*bottom_row_pixel, image::Rgba([229, 229, 229, 255]));
    }

    #[test]
    fn unmapped_glyph_is_a_hard_error_naming_the_codepoint_and_position() {
        let mut p = vt100::Parser::new(1, 1, 0);
        p.process("\u{2726}".as_bytes());
        let err = render_screen(p.screen()).unwrap_err();
        assert_eq!(err, RenderError::Glyph(glyph::GlyphError::Unmapped('\u{2726}'), 0, 0));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p visual-snapshot render::`
Expected: FAIL — `render_screen`/`RenderError` not defined.

- [ ] **Step 3: Implement the render module**

```rust
// tools/visual-snapshot/src/render.rs (above the tests module)
use crate::{color, glyph};
use image::{Rgba, RgbaImage};

const CELL_PX: u32 = 16;
const GLYPH_PX: u32 = 8;
const SCALE: u32 = CELL_PX / GLYPH_PX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    Glyph(glyph::GlyphError, u16, u16),
}

/// Rasterizes a parsed terminal screen to a 2x-upscaled RGBA image,
/// one 16x16 block per cell.
pub fn render_screen(screen: &vt100::Screen) -> Result<RgbaImage, RenderError> {
    let (rows, cols) = screen.size();
    let mut img = RgbaImage::new(cols as u32 * CELL_PX, rows as u32 * CELL_PX);

    for row in 0..rows {
        for col in 0..cols {
            let cell = screen.cell(row, col);
            let (ch, fg, bg, bold, underline, inverse) = match cell {
                Some(c) => (
                    c.contents().chars().next().unwrap_or(' '),
                    color::to_rgb(c.fgcolor()),
                    color::to_rgb(c.bgcolor()),
                    c.bold(),
                    c.underline(),
                    c.inverse(),
                ),
                None => (' ', color::to_rgb(vt100::Color::Default), image::Rgb([0u8, 0, 0]), false, false, false),
            };

            let mut fg = fg;
            let mut bg = bg;
            if bold {
                fg = color::brighten(fg);
            }
            if inverse {
                let (nf, nb) = color::swap(fg, bg);
                fg = nf;
                bg = nb;
            }

            let bitmap = glyph::glyph_for(ch).map_err(|e| RenderError::Glyph(e, row, col))?;

            let ox = col as u32 * CELL_PX;
            let oy = row as u32 * CELL_PX;
            for gy in 0..GLYPH_PX {
                let row_bits = bitmap[gy as usize];
                for gx in 0..GLYPH_PX {
                    let set = (row_bits >> gx) & 1 == 1;
                    let px = if set { fg } else { bg };
                    for sy in 0..SCALE {
                        for sx in 0..SCALE {
                            img.put_pixel(
                                ox + gx * SCALE + sx,
                                oy + gy * SCALE + sy,
                                Rgba([px.0[0], px.0[1], px.0[2], 255]),
                            );
                        }
                    }
                }
            }

            if underline {
                let fg_px = Rgba([fg.0[0], fg.0[1], fg.0[2], 255]);
                for x in 0..CELL_PX {
                    img.put_pixel(ox + x, oy + CELL_PX - 1, fg_px);
                }
            }
        }
    }

    Ok(img)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p visual-snapshot render::`
Expected: PASS (6 tests). `font8x8`'s actual bit-order convention (LSB-first vs MSB-first per row byte) is not confirmed from docs alone — if the `bold_text_brightens_the_foreground` or `plain_text_uses_default_fg_over_default_bg` tests fail because the glyph renders mirrored/blank, flip the bit test (`(row_bits >> gx) & 1` vs `(row_bits >> (7 - gx)) & 1`) and re-run; this is exactly the kind of detail the RGR cycle catches immediately via a failing assertion rather than a silent visual bug.

- [ ] **Step 5: Wire the module in and commit**

Add `pub mod render;` to `tools/visual-snapshot/src/lib.rs`.

```bash
git add tools/visual-snapshot/src/render.rs tools/visual-snapshot/src/lib.rs
git commit -m "feat(tooling): add vt100 Screen to RGBA image rasterizer"
```

---

### Task 6: PNG/GIF encoding

**Files:**
- Create: `tools/visual-snapshot/src/encode.rs`
- Modify: `tools/visual-snapshot/src/lib.rs` (add `pub mod encode;`)

**Interfaces:**
- Produces: `pub fn write_png(img: &image::RgbaImage, path: &std::path::Path) -> Result<(), EncodeError>`, `pub fn write_gif(frames: &[(image::RgbaImage, std::time::Duration)], path: &std::path::Path) -> Result<(), EncodeError>`, `pub enum EncodeError { Io(std::io::Error), Image(image::ImageError) }` — consumed by `main.rs` (Task 10).

- [ ] **Step 1: Write the failing tests**

```rust
// tools/visual-snapshot/src/encode.rs
#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::time::Duration;

    fn solid(w: u32, h: u32, px: Rgba<u8>) -> RgbaImage {
        RgbaImage::from_pixel(w, h, px)
    }

    #[test]
    fn write_png_round_trips_dimensions_and_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frame.png");
        let img = solid(4, 2, Rgba([10, 20, 30, 255]));

        write_png(&img, &path).unwrap();

        let reopened = image::open(&path).unwrap().to_rgba8();
        assert_eq!(reopened.dimensions(), (4, 2));
        assert_eq!(*reopened.get_pixel(0, 0), Rgba([10, 20, 30, 255]));
    }

    #[test]
    fn write_gif_round_trips_frame_count() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("seq.gif");
        let frames = vec![
            (solid(2, 2, Rgba([255, 0, 0, 255])), Duration::from_millis(16)),
            (solid(2, 2, Rgba([0, 255, 0, 255])), Duration::from_millis(150)),
        ];

        write_gif(&frames, &path).unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let decoder = image::codecs::gif::GifDecoder::new(file).unwrap();
        let decoded_frames: Vec<_> = image::AnimationDecoder::into_frames(decoder)
            .collect_frames()
            .unwrap();
        assert_eq!(decoded_frames.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p visual-snapshot encode::`
Expected: FAIL — `write_png`/`write_gif`/`EncodeError` not defined.

- [ ] **Step 3: Implement the encode module**

```rust
// tools/visual-snapshot/src/encode.rs (above the tests module)
use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, RgbaImage};
use std::fs::File;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub enum EncodeError {
    Io(std::io::Error),
    Image(image::ImageError),
}

impl From<std::io::Error> for EncodeError {
    fn from(e: std::io::Error) -> Self {
        EncodeError::Io(e)
    }
}

impl From<image::ImageError> for EncodeError {
    fn from(e: image::ImageError) -> Self {
        EncodeError::Image(e)
    }
}

/// Writes a single frame as a PNG.
pub fn write_png(img: &RgbaImage, path: &Path) -> Result<(), EncodeError> {
    img.save(path)?;
    Ok(())
}

/// Writes multiple frames as an animated GIF, each held for its paired duration.
pub fn write_gif(frames: &[(RgbaImage, Duration)], path: &Path) -> Result<(), EncodeError> {
    let file = File::create(path)?;
    let mut encoder = GifEncoder::new(file);
    for (img, duration) in frames {
        let delay = Delay::from_saturating_duration(*duration);
        let frame = Frame::from_parts(img.clone(), 0, 0, delay);
        encoder.encode_frame(frame)?;
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p visual-snapshot encode::`
Expected: PASS (2 tests).

- [ ] **Step 5: Wire the module in and commit**

Add `pub mod encode;` to `tools/visual-snapshot/src/lib.rs`.

```bash
git add tools/visual-snapshot/src/encode.rs tools/visual-snapshot/src/lib.rs
git commit -m "feat(tooling): add PNG/GIF frame encoding"
```

---

### Task 7: Script parsing

**Files:**
- Create: `tools/visual-snapshot/src/script.rs`
- Modify: `tools/visual-snapshot/src/lib.rs` (add `pub mod script;`)

**Interfaces:**
- Produces: `pub enum Step { Wait { wait_ms: u64 }, Key { key: String } }` (derives `Debug, Clone, PartialEq, serde::Deserialize`), `pub fn parse_script(path: &std::path::Path) -> Result<Vec<Step>, ScriptError>`, `pub enum ScriptError { Io(std::io::Error), Json(serde_json::Error) }` — consumed by `main.rs` (Task 10) and `pty.rs`'s tests (Task 9).

- [ ] **Step 1: Write the failing tests**

```rust
// tools/visual-snapshot/src/script.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_mix_of_wait_and_key_steps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(&path, r#"[{"wait_ms":16},{"key":"Right"},{"wait_ms":150},{"key":"Enter"}]"#).unwrap();

        let steps = parse_script(&path).unwrap();

        assert_eq!(
            steps,
            vec![
                Step::Wait { wait_ms: 16 },
                Step::Key { key: "Right".to_string() },
                Step::Wait { wait_ms: 150 },
                Step::Key { key: "Enter".to_string() },
            ]
        );
    }

    #[test]
    fn empty_script_parses_to_an_empty_vec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(&path, "[]").unwrap();

        assert_eq!(parse_script(&path).unwrap(), Vec::new());
    }

    #[test]
    fn malformed_json_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(&path, "not json").unwrap();

        assert!(matches!(parse_script(&path), Err(ScriptError::Json(_))));
    }

    #[test]
    fn missing_file_is_an_error() {
        let missing = std::path::Path::new("/does/not/exist.json");
        assert!(matches!(parse_script(missing), Err(ScriptError::Io(_))));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p visual-snapshot script::`
Expected: FAIL — `Step`/`parse_script`/`ScriptError` not defined.

- [ ] **Step 3: Implement the script module**

```rust
// tools/visual-snapshot/src/script.rs (above the tests module)
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Step {
    Wait { wait_ms: u64 },
    Key { key: String },
}

#[derive(Debug)]
pub enum ScriptError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl From<std::io::Error> for ScriptError {
    fn from(e: std::io::Error) -> Self {
        ScriptError::Io(e)
    }
}

impl From<serde_json::Error> for ScriptError {
    fn from(e: serde_json::Error) -> Self {
        ScriptError::Json(e)
    }
}

/// Reads and parses a snapshot script: a flat JSON array of `{"wait_ms": N}`
/// and `{"key": "Name"}` steps.
pub fn parse_script(path: &Path) -> Result<Vec<Step>, ScriptError> {
    let contents = std::fs::read_to_string(path)?;
    let steps = serde_json::from_str(&contents)?;
    Ok(steps)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p visual-snapshot script::`
Expected: PASS (4 tests).

- [ ] **Step 5: Wire the module in and commit**

Add `pub mod script;` to `tools/visual-snapshot/src/lib.rs`.

```bash
git add tools/visual-snapshot/src/script.rs tools/visual-snapshot/src/lib.rs
git commit -m "feat(tooling): add snapshot script JSON parsing"
```

---

### Task 8: PTY spawn infrastructure + single-frame capture

**Files:**
- Create: `tools/visual-snapshot/src/pty.rs`
- Create: `tools/visual-snapshot/examples/echo_key.rs`
- Modify: `tools/visual-snapshot/src/lib.rs` (add `pub mod pty;`)
- Test: `tools/visual-snapshot/tests/pty_roundtrip.rs`

**Interfaces:**
- Consumes: `render::render_screen` (Task 5) — used internally by `capture_frame`.
- Produces: `pub fn build_example(name: &str) -> Result<std::path::PathBuf, PtyError>`, `pub struct Session { .. }` with `pub fn spawn(binary: &std::path::Path, rows: u16, cols: u16) -> Result<Session, PtyError>`, `pub fn capture_frame(&mut self) -> Result<image::RgbaImage, PtyError>`, `pub enum PtyError { Io(std::io::Error), Pty(String), Render(render::RenderError) }` — `Session::spawn`/`capture_frame` are consumed directly by this task's own tests and extended by Task 9's `run_script`.

This is the one module whose core mechanism (spawning a real OS pseudo-console) can't be verified from docs.rs alone — a fixture binary and a real integration test are written first, per `development-conventions.md`'s reasoning that this doesn't qualify for the real-TTY `#[ignore]` exception (a `portable-pty` pseudo-console is created by our own process, not borrowed from an already-real host TTY).

- [ ] **Step 1: Write the fixture binary**

```rust
// tools/visual-snapshot/examples/echo_key.rs
use crossterm::event::{self, Event};
use crossterm::terminal;
use std::io::Write;

fn main() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    loop {
        if let Event::Key(key) = event::read()? {
            write!(out, "{:?}", key.code)?;
            out.flush()?;
            if key.code == event::KeyCode::Esc {
                break;
            }
        }
    }
    terminal::disable_raw_mode()?;
    Ok(())
}
```

This is a test fixture, not application code — exempt from TDD per the examples/demos exception already named in `development-conventions.md`; its correctness is checked by the integration test in Step 4, not by a unit test of its own.

- [ ] **Step 2: Verify the fixture builds**

Run: `cargo build -p visual-snapshot --example echo_key`
Expected: builds with no errors, producing `target/debug/examples/echo_key` (or `echo_key.exe` on Windows).

- [ ] **Step 3: Write the failing integration test**

```rust
// tools/visual-snapshot/tests/pty_roundtrip.rs
use std::path::PathBuf;
use visual_snapshot::pty::Session;

fn echo_key_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/examples/echo_key");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

#[test]
fn spawning_and_capturing_one_frame_shows_the_process_alive() {
    let mut session = Session::spawn(&echo_key_binary(), 5, 40).unwrap();
    // No input sent yet — the fixture is blocked on event::read(), so
    // the initial frame should just be a blank screen at the right size.
    let frame = session.capture_frame().unwrap();
    assert_eq!(frame.width(), 40 * 16);
    assert_eq!(frame.height(), 5 * 16);
}
```

`visual_snapshot::pty::Session` requires `lib.rs` to expose the crate's modules under the crate name `visual_snapshot` (matches `[package] name = "visual-snapshot"` from Task 1 — Cargo maps the hyphenated package name to the underscored `visual_snapshot` module path automatically).

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p visual-snapshot --test pty_roundtrip`
Expected: FAIL — `Session`/`spawn`/`capture_frame` not defined (compile error).

- [ ] **Step 5: Implement `pty.rs`'s spawn + single-frame capture**

```rust
// tools/visual-snapshot/src/pty.rs
use crate::render::{self, RenderError};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum PtyError {
    Io(std::io::Error),
    Pty(String),
    Render(RenderError),
}

impl From<std::io::Error> for PtyError {
    fn from(e: std::io::Error) -> Self {
        PtyError::Io(e)
    }
}

impl From<RenderError> for PtyError {
    fn from(e: RenderError) -> Self {
        PtyError::Render(e)
    }
}

/// Builds `cargo build --example <name>` and returns the resulting
/// binary's path (relative to the workspace's shared `target/` dir).
pub fn build_example(name: &str) -> Result<PathBuf, PtyError> {
    let status = StdCommand::new("cargo")
        .args(["build", "--example", name])
        .status()?;
    if !status.success() {
        return Err(PtyError::Pty(format!("cargo build --example {name} failed")));
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/examples");
    path.push(name);
    if cfg!(windows) {
        path.set_extension("exe");
    }
    Ok(path)
}

pub const SETTLE_DELAY: Duration = Duration::from_millis(100);

/// An active PTY-attached child process plus a background thread
/// continuously draining its output into a shared buffer.
pub struct Session {
    parser: vt100::Parser,
    writer: Box<dyn std::io::Write + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Session {
    /// Spawns `binary` under a new pseudo-console of the given size and
    /// starts a background reader thread.
    pub fn spawn(binary: &Path, rows: u16, cols: u16) -> Result<Session, PtyError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let cmd = CommandBuilder::new(binary);
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Pty(e.to_string()))?;
        // Drop the slave handle once the child is spawned so the master
        // side can observe EOF when the child exits.
        drop(pair.slave);

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Pty(e.to_string()))?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_writer = Arc::clone(&output);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => output_writer.lock().unwrap().extend_from_slice(&buf[..n]),
                    Err(_) => break,
                }
            }
        });

        Ok(Session {
            parser: vt100::Parser::new(rows, cols, 0),
            writer,
            output,
            master: pair.master,
            child,
        })
    }

    /// Waits `SETTLE_DELAY`, drains whatever output has arrived since
    /// the last capture into the parser, and rasterizes the current
    /// screen state.
    pub fn capture_frame(&mut self) -> Result<image::RgbaImage, PtyError> {
        thread::sleep(SETTLE_DELAY);
        let pending: Vec<u8> = {
            let mut buf = self.output.lock().unwrap();
            std::mem::take(&mut *buf)
        };
        self.parser.process(&pending);
        Ok(render::render_screen(self.parser.screen())?)
    }

    /// Writes raw bytes into the pseudo-console's input handle.
    pub fn send(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Terminates the child process. Safe to call even after the child
    /// has already exited on its own. `self.master` is intentionally
    /// unused here — it exists as a `Session` field solely to stay
    /// alive (and keep the pseudo-console open) for as long as
    /// `Session` itself does; ownership does that on its own, no
    /// explicit method call needed.
    pub fn kill(&mut self) -> Result<(), PtyError> {
        let _ = self.child.kill();
        Ok(())
    }
}
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo build -p visual-snapshot --example echo_key && cargo test -p visual-snapshot --test pty_roundtrip`
Expected: PASS. If `portable-pty`'s trait object types (`MasterPty`, `Child`) don't satisfy `Send`/`Sync` the way this signature assumes, or method names differ from what's shown here (the exact `Child::kill` signature wasn't confirmed from docs.rs — only that it's inherited from `ChildKiller`), the compiler error will show the real trait bounds/method signature — adjust `Session`'s field types and `kill`'s body accordingly; this is exactly the kind of gap the plan's "Open questions... verify during implementation" framing anticipated.

- [ ] **Step 7: Wire the module in and commit**

Add `pub mod pty;` to `tools/visual-snapshot/src/lib.rs`.

```bash
git add tools/visual-snapshot/src/pty.rs tools/visual-snapshot/examples/echo_key.rs tools/visual-snapshot/tests/pty_roundtrip.rs tools/visual-snapshot/src/lib.rs
git commit -m "feat(tooling): add PTY spawn and single-frame capture"
```

---

### Task 9: Script-driven multi-step capture

**Files:**
- Modify: `tools/visual-snapshot/src/pty.rs` (add `run_script`)
- Modify: `tools/visual-snapshot/tests/pty_roundtrip.rs`

**Interfaces:**
- Consumes: `keys::encode_key` (Task 2), `script::Step` (Task 7), `Session::send`/`capture_frame` (Task 8).
- Produces: `pub fn run_script(binary: &std::path::Path, rows: u16, cols: u16, steps: &[script::Step]) -> Result<Vec<(image::RgbaImage, std::time::Duration)>, PtyError>` — consumed by `main.rs` (Task 10).

- [ ] **Step 1: Write the failing integration test**

```rust
// tools/visual-snapshot/tests/pty_roundtrip.rs (append)
use visual_snapshot::pty::run_script;
use visual_snapshot::script::Step;

#[test]
fn a_key_step_actually_reaches_the_child_process() {
    let steps = vec![
        Step::Key { key: "a".to_string() },
        Step::Wait { wait_ms: 16 },
        Step::Key { key: "Esc".to_string() },
    ];

    let frames = run_script(&echo_key_binary(), 5, 40, &steps).unwrap();

    // Initial frame + one per step.
    assert_eq!(frames.len(), 4);
    // The frame captured after sending "a" should show the fixture's
    // echoed `KeyCode::Char('a')` debug text somewhere on screen —
    // checked indirectly via a non-blank pixel outside the top-left
    // origin, since asserting exact glyph pixels here would duplicate
    // render.rs's own tests.
    let after_a = &frames[1].0;
    let any_non_background = after_a.pixels().any(|p| *p != image::Rgba([0, 0, 0, 255]));
    assert!(any_non_background, "expected the echoed key text to draw something");
}

#[test]
fn frame_durations_match_each_steps_own_timing() {
    let steps = vec![
        Step::Wait { wait_ms: 250 },
        Step::Key { key: "Esc".to_string() },
    ];

    let frames = run_script(&echo_key_binary(), 5, 40, &steps).unwrap();

    assert_eq!(frames[0].1, std::time::Duration::from_millis(0)); // initial frame
    assert_eq!(frames[1].1, std::time::Duration::from_millis(250)); // Wait step
    assert_eq!(frames[2].1, std::time::Duration::from_millis(150)); // Key step, fixed duration
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p visual-snapshot --test pty_roundtrip`
Expected: FAIL — `run_script` not defined.

- [ ] **Step 3: Implement `run_script`**

```rust
// tools/visual-snapshot/src/pty.rs (add below the `impl Session` block)
use crate::keys;
use crate::script::Step;
use std::time::Duration;

const KEY_STEP_DISPLAY_DURATION: Duration = Duration::from_millis(150);

/// Spawns `binary`, drives it through `steps` (real key bytes / real
/// wall-clock waits), and returns one rendered frame per step plus an
/// initial frame captured before any step runs.
pub fn run_script(
    binary: &Path,
    rows: u16,
    cols: u16,
    steps: &[Step],
) -> Result<Vec<(image::RgbaImage, Duration)>, PtyError> {
    let mut session = Session::spawn(binary, rows, cols)?;
    let mut frames = Vec::with_capacity(steps.len() + 1);

    frames.push((session.capture_frame()?, Duration::from_millis(0)));

    for step in steps {
        let duration = match step {
            Step::Wait { wait_ms } => {
                std::thread::sleep(Duration::from_millis(*wait_ms));
                Duration::from_millis(*wait_ms)
            }
            Step::Key { key } => {
                let bytes = keys::encode_key(key)
                    .map_err(|e| PtyError::Pty(format!("{e:?}")))?;
                session.send(&bytes)?;
                KEY_STEP_DISPLAY_DURATION
            }
        };
        frames.push((session.capture_frame()?, duration));
    }

    session.kill()?;
    Ok(frames)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p visual-snapshot --test pty_roundtrip`
Expected: PASS (3 tests total in this file). Timing assertions have real wall-clock slop from thread scheduling and the 100ms settle delay baked into `capture_frame` — if `frame_durations_match_each_steps_own_timing` is flaky on exact equality, that's a real signal the settle-delay/timing model needs a tolerance band rather than exact-match assertions; adjust the test to an inequality/range check (e.g. `>= Duration::from_millis(250)`) rather than deleting the coverage.

- [ ] **Step 5: Commit**

```bash
git add tools/visual-snapshot/src/pty.rs tools/visual-snapshot/tests/pty_roundtrip.rs
git commit -m "feat(tooling): drive PTY sessions with scripted key/wait steps"
```

---

### Task 10: CLI wiring

**Files:**
- Modify: `tools/visual-snapshot/src/main.rs`

**Interfaces:**
- Consumes: `pty::build_example`, `pty::run_script` (Tasks 8-9); `script::parse_script` (Task 7); `encode::write_png`/`write_gif` (Task 6).
- Produces: the `visual-snapshot` CLI binary — the deliverable reviewers actually invoke (`cargo run -p visual-snapshot -- --example <name> --size <cols>x<rows> --script <path> --out <path>`).

- [ ] **Step 1: Write the failing test for the size-string parser**

```rust
// tools/visual-snapshot/src/main.rs (top of file, before fn main)
fn parse_size(s: &str) -> Result<(u16, u16), String> {
    let (cols, rows) = s
        .split_once('x')
        .ok_or_else(|| format!("expected COLSxROWS, got {s:?}"))?;
    let cols: u16 = cols.parse().map_err(|_| format!("bad cols in {s:?}"))?;
    let rows: u16 = rows.parse().map_err(|_| format!("bad rows in {s:?}"))?;
    Ok((cols, rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cols_x_rows() {
        assert_eq!(parse_size("120x40"), Ok((120, 40)));
    }

    #[test]
    fn rejects_missing_separator() {
        assert!(parse_size("12040").is_err());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p visual-snapshot --bin visual-snapshot`
Expected: FAIL — `parse_size` not defined (it's written above but `main` doesn't exist yet in a state that compiles as a binary with no `main` conflicts; more precisely, expect the two new tests to fail before Step 3's `main` exists to tie the crate together — if `cargo test` already passes because `parse_size` alone compiles, proceed directly to confirming both tests pass, then continue to Step 3, which is legitimate: this step's real purpose is confirming the size parser's behavior before wiring it into `main`).

- [ ] **Step 3: Implement the full CLI**

```rust
// tools/visual-snapshot/src/main.rs (replace the placeholder main)
use clap::Parser as ClapParser;
use visual_snapshot::{encode, pty, script};

#[derive(ClapParser)]
struct Args {
    #[arg(long)]
    example: String,
    #[arg(long, default_value = "80x24")]
    size: String,
    #[arg(long)]
    script: std::path::PathBuf,
    #[arg(long)]
    out: std::path::PathBuf,
}

fn parse_size(s: &str) -> Result<(u16, u16), String> {
    let (cols, rows) = s
        .split_once('x')
        .ok_or_else(|| format!("expected COLSxROWS, got {s:?}"))?;
    let cols: u16 = cols.parse().map_err(|_| format!("bad cols in {s:?}"))?;
    let rows: u16 = rows.parse().map_err(|_| format!("bad rows in {s:?}"))?;
    Ok((cols, rows))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let (cols, rows) = parse_size(&args.size)?;

    let binary = pty::build_example(&args.example)?;
    let steps = script::parse_script(&args.script)?;
    let frames = pty::run_script(&binary, rows, cols, &steps)?;

    if frames.len() == 1 {
        encode::write_png(&frames[0].0, &args.out)?;
    } else {
        encode::write_gif(&frames, &args.out)?;
    }

    println!("wrote {} frame(s) to {}", frames.len(), args.out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cols_x_rows() {
        assert_eq!(parse_size("120x40"), Ok((120, 40)));
    }

    #[test]
    fn rejects_missing_separator() {
        assert!(parse_size("12040").is_err());
    }
}
```

`pty::PtyError`, `script::ScriptError`, and `encode::EncodeError` all need to implement `std::error::Error` (plus `std::fmt::Display`, which `Error` requires) for the `?` operator to convert them into `Box<dyn std::error::Error>` in `main`. Add these impls to each error enum now:

```rust
// tools/visual-snapshot/src/pty.rs (add near PtyError's definition)
impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PtyError {}
```

```rust
// tools/visual-snapshot/src/script.rs (add near ScriptError's definition)
impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ScriptError {}
```

```rust
// tools/visual-snapshot/src/encode.rs (add near EncodeError's definition)
impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for EncodeError {}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p visual-snapshot --bin visual-snapshot`
Expected: PASS (2 tests). Then: `cargo build -p visual-snapshot` to confirm the full binary compiles with the new error trait impls.

- [ ] **Step 5: Commit**

```bash
git add tools/visual-snapshot/src/main.rs tools/visual-snapshot/src/pty.rs tools/visual-snapshot/src/script.rs tools/visual-snapshot/src/encode.rs
git commit -m "feat(tooling): wire visual-snapshot CLI end to end"
```

---

### Task 11: Wire into the review workflow

**Files:**
- Modify: `.claude/rules/development-conventions.md`

**Tag: docs-only — TDD exception per `development-conventions.md`'s own existing rule.**

- [ ] **Step 1: Add a new subsection near "Real-TTY tests"**

Insert after the existing "Real-TTY tests" subsection in `.claude/rules/development-conventions.md`:

```markdown
## Visual review

Any task or final code review touching rendering-affecting code
(`src/effects.rs`, `src/particles.rs`, `src/transition.rs`,
`src/widgets/`, `src/canvas.rs`, `src/glitch.rs`, or an example's
`view()`/`on_tick()`) must run `tools/visual-snapshot` against the
affected example(s) and `Read` the resulting PNG/GIF before approving —
not optional, and "reasoned through it, no PTY available" is no longer
an acceptable substitute now that this tool exists. See
`docs/design/specs/core/2026-08-09-visual-snapshot-tooling-design.md`.

Record which snapshots were reviewed in the PR template's existing
freeform Verification section
(`.claude/templates/github/PULL_REQUEST_TEMPLATE.md`), the same pattern
already used for real-TTY test results below.
```

- [ ] **Step 2: Verify the file still renders sensibly**

Run: view the diff (`git diff .claude/rules/development-conventions.md`) and confirm the new section sits between "Real-TTY tests" and "Commit conventions" without breaking heading levels.

- [ ] **Step 3: Commit**

```bash
git add .claude/rules/development-conventions.md
git commit -m "docs(core): require visual-snapshot in rendering-affecting reviews"
```

---

### Task 12: End-to-end verification against a real example

**Files:** none modified — this task is verification only.

- [ ] **Step 1: Write a real script and run the tool against `launcher`**

```bash
echo '[{"wait_ms":200}]' > /tmp/launcher-idle.json
cargo run -p visual-snapshot -- --example launcher --size 120x40 --script /tmp/launcher-idle.json --out /tmp/launcher-idle.png
```

Expected: a PNG is written; `Read` it and confirm it shows the launcher's nexus/portals layout, not a blank or garbled image.

- [ ] **Step 2: Run against `omnitrix` to confirm the expected hard error**

```bash
echo '[{"wait_ms":500}]' > /tmp/omnitrix-idle.json
cargo run -p visual-snapshot -- --example omnitrix --size 120x40 --script /tmp/omnitrix-idle.json --out /tmp/omnitrix-idle.png
```

If `EnergyCore`'s charged state is on-screen in this scenario, expected: the tool exits with an error naming `'\u{2726}'` (per the spec's confirmed `font8x8` gap) rather than a silently wrong image. If the scenario doesn't happen to reach the charged state, note that in the task's completion summary rather than treating it as a pass/fail signal either way — this step is about confirming the hard-error path fires correctly when it's reachable, not about forcing that state.

- [ ] **Step 3: Run a multi-step script and confirm a GIF comes out**

```bash
echo '[{"wait_ms":16},{"key":"Right"},{"wait_ms":16},{"key":"Right"}]' > /tmp/launcher-nav.json
cargo run -p visual-snapshot -- --example launcher --size 120x40 --script /tmp/launcher-nav.json --out /tmp/launcher-nav.gif
```

Expected: a 5-frame animated GIF; `Read` it and confirm the focused portal visibly changes across frames.

- [ ] **Step 4: Record results**

Note the outcome of Steps 1-3 (which images were produced, whether the hard-error path fired, whether navigation was visible across GIF frames) in the task's completion summary — this is the plan's final verification gate before the Arc is considered done per `.claude/rules/git-github-standards.md`.

---

## Self-Review Notes

- **Spec coverage:** every "In scope" bullet from the design spec maps to a task — CLI (Task 10), workspace/`[workspace]` (Task 1), script format (Task 7), PTY driving (Tasks 8-9), rendering (Tasks 3-5), PNG/GIF output (Task 6), review-workflow wiring (Task 11). The spec's confirmed `vt100`/`font8x8` findings are reflected in Task 4's and Task 5's tests directly (the `dingbat_star_is_unmapped` and hard-error tests), not just asserted in prose.
- **Type consistency checked:** `render::RenderError` (Task 5) is the type `pty::PtyError::Render` (Task 8) wraps; `script::Step` (Task 7) is the exact type `pty::run_script` (Task 9) consumes; `keys::KeyEncodeError` (Task 2) is formatted into `PtyError::Pty` in Task 9 rather than given its own `PtyError` variant, since `pty.rs` only needs to report it, not match on it further downstream.
- **No placeholders:** every step above shows real code with the actual signatures found on docs.rs, or explicitly says what to check via `cargo doc`/the compiler where a signature detail wasn't independently confirmed (font8x8's exact `UnicodeFonts::get` shape, `vt100`'s exact `Color`/`Cell` shape at the resolved patch version, `portable-pty`'s `Child`/`ChildKiller` trait bounds) — these are named as concrete verification steps within their tasks, not left as unstated assumptions.
