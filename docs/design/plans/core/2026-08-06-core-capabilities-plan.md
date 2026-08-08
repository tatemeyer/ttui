# Shared Core Capabilities (Arc 0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan, with the deviation recorded in "Concurrency" below. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the six Shared Core Capabilities from
`docs/design/specs/2026-08-06-core-capabilities-design.md` (GitHub
issues #34-#39, tracking #40): `Cell` style/bold, a screen-shake
transform, an easing/tween helper, a particle system, a transition
state helper, and an audio-sink trait seam (no new dependency).

**Architecture:** Six independent tasks. Tasks 2-6 each add exactly one
new file plus one `pub mod` line in `src/lib.rs`. Task 1 touches
`src/buffer.rs`, `src/terminal.rs`, four widget files, and
`examples/smash_crabs.rs` (mechanical literal-site updates forced by a
new struct field). No task's *tests* depend on another task's code —
confirmed independent per the design spec.

**Tech Stack:** Rust, `crossterm` (unchanged). No new dependencies —
Task 6 explicitly does not add `rodio` (see spec's "explicit dependency
decision").

**Concurrency:** unlike this skill's default (sequential-only
implementers to avoid conflicts), Tasks 1-6 are dispatched
concurrently, each to its own git-worktree-isolated implementer, since
they are file-disjoint except for `src/lib.rs`'s six independent
one-line `pub mod` additions — reconciled by hand at integration, not
routed through the fix loop. Each task still gets its own fresh
implementer, its own task review, and its own ledger entry, per this
skill's core loop; only the "one implementer at a time" rule is
relaxed, and only because of the worktree isolation.

## Global Constraints

- TDD mandatory for all six tasks (all `coding`-tagged, no exception
  applies) — except Task 1's `terminal.rs` half, which is real-TTY-
  dependent (verified manually via `cargo run --example`, not unit
  tested), per `.claude/rules/development-conventions.md`.
- Inline `#[cfg(test)] mod tests` per module — no new `tests/` directory.
- `cargo fmt` / `cargo clippy --all-targets` clean in every task.
- No new dependencies added anywhere in this plan.
- Every new file gets exactly one corresponding `pub mod <name>;` line
  added to `src/lib.rs`.

---

### Task 1: `Cell` style (bold)

**Files:** `src/buffer.rs`, `src/terminal.rs`,
`src/widgets/text.rs`, `src/widgets/block.rs`, `src/widgets/list.rs`,
`src/widgets/table.rs`, `examples/smash_crabs.rs`.

**Interfaces produced:**
```rust
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CellStyle {
    pub bold: bool,
}
```
`Cell` gains `pub style: CellStyle`; `Cell::default()` sets
`style: CellStyle::default()`.

- [ ] **Write failing tests** in `src/buffer.rs`'s existing `mod tests`:
  - `CellStyle::default().bold` is `false`.
  - `Cell::default().style` equals `CellStyle::default()`.
  - Two cells identical except `style.bold` are unequal (`PartialEq`)
    — locks in that `diff()` will treat a bold-only change as a real
    diff (no new `diff()` code needed; this is a consequence of
    `#[derive(PartialEq)]` already covering the new field, verified by
    a test rather than assumed).
- [ ] Run `cargo test --lib buffer::tests` — confirm failure to
  compile (`CellStyle` doesn't exist yet).
- [ ] **Implement:** add `CellStyle` above `Cell` in `src/buffer.rs`;
  add the `style` field to `Cell`; add it to `Cell::default()`.
- [ ] **Fix every existing exhaustive `Cell { .. }` literal** that the
  compiler now flags as missing a field — append `, ..Default::default()`
  to each. Expected sites (confirm via `cargo build` errors, this list
  may not be exhaustive): `src/widgets/text.rs` (~1 site),
  `src/widgets/block.rs` (~9 sites), `src/widgets/list.rs` (~2 sites),
  `src/widgets/table.rs` (~1 site), `examples/smash_crabs.rs` (2 sites:
  `paint_background`'s `cell` and `paint_effects`'s `flash`). Do not
  change any other behavior in these files — this is a purely additive,
  mechanical fix.
- [ ] Run `cargo test --lib` — all tests pass, including the new ones.
- [ ] **`terminal.rs` bold wiring** (no unit test — real-TTY-dependent
  per the TDD exception): in `draw_diff`, before the existing `Print`,
  add `SetAttribute(crossterm::style::Attribute::Reset)`, then if
  `d.cell.style.bold`, `SetAttribute(crossterm::style::Attribute::Bold)`.
  Import `crossterm::style::{Attribute, SetAttribute}` alongside the
  existing `style` imports.
- [ ] Run `cargo build --examples` — all three examples still compile.
- [ ] `cargo fmt && cargo clippy --all-targets` — clean.
- [ ] Manual smoke check: `cargo run --example omnitrix` and
  `cargo run --example smash_crabs` still render identically to before
  (no example sets `style.bold` yet, so this is a no-op visually —
  confirms the change is additive).
- [ ] Commit.

---

### Task 2: screen-shake helper

**Files:** new `src/effects.rs`; `src/lib.rs` (`pub mod effects;`).

**Interfaces produced:** `pub fn shake(buf: &Buffer, dx: i16, dy: i16) -> Buffer`.
**Consumes:** `crate::buffer::{Buffer, Cell}` (existing, unchanged
public API — `Buffer::new`, `.width`, `.height`, `.get`, `.set`).

- [ ] **Write failing tests** in a new `#[cfg(test)] mod tests` in
  `src/effects.rs`:
  - `shake` with `dx=0, dy=0` on a buffer with one non-default cell
    returns an unchanged buffer (same cell at the same position).
  - `shake` with `dx=1, dy=0` moves a cell at `(0, 0)` to `(1, 0)` in
    the output, and `(0, 0)` in the output is `Cell::default()`.
  - `shake` with `dy=-1` moves a cell at `(0, 1)` to `(0, 0)`.
  - `shake` with an offset large enough to move every original cell out
    of bounds (e.g. `dx` equal to the buffer's width) returns a buffer
    where every cell is `Cell::default()`.
- [ ] Run `cargo test --lib effects::tests` — confirm compile failure
  (module doesn't exist).
- [ ] **Implement** `src/effects.rs`:
```rust
use crate::buffer::Buffer;

pub fn shake(buf: &Buffer, dx: i16, dy: i16) -> Buffer {
    let mut out = Buffer::new(buf.width, buf.height);
    for y in 0..buf.height {
        for x in 0..buf.width {
            let src_x = x as i32 - dx as i32;
            let src_y = y as i32 - dy as i32;
            if src_x >= 0
                && src_y >= 0
                && (src_x as u16) < buf.width
                && (src_y as u16) < buf.height
            {
                out.set(x, y, buf.get(src_x as u16, src_y as u16).clone());
            }
        }
    }
    out
}
```
- [ ] Add `pub mod effects;` to `src/lib.rs`.
- [ ] Run `cargo test --lib` — all green.
- [ ] `cargo fmt && cargo clippy --all-targets` — clean.
- [ ] Commit.

---

### Task 3: easing/tween helper

**Files:** new `src/easing.rs`; `src/lib.rs` (`pub mod easing;`).

**Interfaces produced:**
```rust
pub fn lerp(start: f32, end: f32, t: f32) -> f32
pub fn ease_out(start: f32, end: f32, t: f32) -> f32
pub fn progress(elapsed: std::time::Duration, duration: std::time::Duration) -> f32
```

- [ ] **Write failing tests** in a new `#[cfg(test)] mod tests` in
  `src/easing.rs`:
  - `lerp(0.0, 10.0, 0.0) == 0.0`; `lerp(0.0, 10.0, 1.0) == 10.0`;
    `lerp(0.0, 10.0, 0.5) == 5.0`.
  - `lerp` clamps: `lerp(0.0, 10.0, -1.0) == 0.0`;
    `lerp(0.0, 10.0, 2.0) == 10.0`.
  - `ease_out(0.0, 10.0, 0.0) == 0.0`; `ease_out(0.0, 10.0, 1.0) == 10.0`.
  - `ease_out(0.0, 10.0, 0.5) > lerp(0.0, 10.0, 0.5)` (front-loaded —
    ease-out moves faster than linear in the first half).
  - `progress(Duration::ZERO, Duration::from_secs(1)) == 0.0`.
  - `progress(Duration::from_secs(1), Duration::from_secs(1)) == 1.0`.
  - `progress(Duration::from_millis(500), Duration::from_secs(1)) == 0.5`.
  - `progress(Duration::from_secs(2), Duration::from_secs(1)) == 1.0`
    (clamped, elapsed past duration).
  - `progress(Duration::from_secs(1), Duration::ZERO) == 1.0`
    (zero-duration edge case).
- [ ] Run `cargo test --lib easing::tests` — confirm compile failure.
- [ ] **Implement** `src/easing.rs`:
```rust
use std::time::Duration;

pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    start + (end - start) * t
}

pub fn ease_out(start: f32, end: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    lerp(start, end, eased)
}

pub fn progress(elapsed: Duration, duration: Duration) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}
```
- [ ] Add `pub mod easing;` to `src/lib.rs`.
- [ ] Run `cargo test --lib` — all green.
- [ ] `cargo fmt && cargo clippy --all-targets` — clean.
- [ ] Commit.

---

### Task 4: particle system

**Files:** new `src/particles.rs`; `src/lib.rs` (`pub mod particles;`).

**Interfaces produced:** `Particle`, `ParticleSystem` (see spec for
full field/method list).
**Consumes:** `crate::buffer::{Buffer, Cell}`, `crossterm::style::Color`.

**Note:** the `Cell` literal in `render()` must use
`, ..Default::default()` even though this task branches before Task 1
lands — see spec's Task 4 section for why (keeps this file compiling
unchanged regardless of merge order against Task 1's new `style` field).

- [ ] **Write failing tests** in a new `#[cfg(test)] mod tests` in
  `src/particles.rs`:
  - `ParticleSystem::new()` is empty (`len() == 0`, `is_empty()`).
  - `spawn()` increases `len()` by 1.
  - `update(elapsed)` moves a particle's `x`/`y` by `vx * elapsed`,
    `vy * elapsed` (use exact values, e.g. `vx=2.0, vy=0.0`, elapsed
    `500ms` → `x` advances by `1.0`).
  - `update(elapsed)` ages a particle by `elapsed`, and removes it
    (drops from the system, `len()` decreases) once
    `age >= lifetime` — spawn a particle with `lifetime =
    Duration::from_millis(100)`, call `update` with `elapsed =
    Duration::from_millis(150)`, assert `len() == 0` afterward.
  - `update` retains particles whose `age < lifetime` — same setup
    with `elapsed = Duration::from_millis(50)`, assert `len() == 1`.
  - `render(&mut buf)` writes the particle's `symbol`/`color` at its
    rounded `(x, y)` position into the buffer.
  - `render` skips a particle whose rounded position falls outside the
    buffer's bounds (negative or `>=` width/height) — buffer is
    unchanged (still `Cell::default()`) at every cell.
- [ ] Run `cargo test --lib particles::tests` — confirm compile
  failure.
- [ ] **Implement** `src/particles.rs`:
```rust
use crate::buffer::{Buffer, Cell};
use crossterm::style::Color;
use std::time::Duration;

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub symbol: char,
    pub color: Color,
    pub lifetime: Duration,
    pub age: Duration,
}

impl Particle {
    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }
}

#[derive(Default)]
pub struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem::default()
    }

    pub fn spawn(&mut self, p: Particle) {
        self.particles.push(p);
    }

    pub fn update(&mut self, elapsed: Duration) {
        let dt = elapsed.as_secs_f32();
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.age += elapsed;
        }
        self.particles.retain(|p| p.is_alive());
    }

    pub fn render(&self, buf: &mut Buffer) {
        for p in &self.particles {
            let x = p.x.round();
            let y = p.y.round();
            if x >= 0.0 && y >= 0.0 && (x as u16) < buf.width && (y as u16) < buf.height {
                buf.set(
                    x as u16,
                    y as u16,
                    Cell {
                        symbol: p.symbol,
                        fg: p.color,
                        bg: Color::Reset,
                        ..Default::default()
                    },
                );
            }
        }
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }
}
```
- [ ] Add `pub mod particles;` to `src/lib.rs`.
- [ ] Run `cargo test --lib` — all green.
- [ ] `cargo fmt && cargo clippy --all-targets` — clean.
- [ ] Commit.

---

### Task 5: transition hook

**Files:** new `src/transition.rs`; `src/lib.rs` (`pub mod transition;`).

**Interfaces produced:** `Transition` (see spec's Task 5 section for
the full method list and the rationale for it being a plain state
struct, not an `App` trait addition).

- [ ] **Write failing tests** in a new `#[cfg(test)] mod tests` in
  `src/transition.rs`:
  - `Transition::start(duration)` begins at `progress() == 0.0` and
    `is_complete() == false`.
  - `tick(elapsed)` advances `progress()` proportionally — e.g.
    `start(Duration::from_secs(1))`, `tick(Duration::from_millis(250))`,
    `progress() == 0.25`.
  - `tick` clamps: ticking past the total duration does not push
    `progress()` above `1.0` — e.g. `start(Duration::from_millis(100))`,
    `tick(Duration::from_millis(200))`, `progress() == 1.0`.
  - `is_complete()` becomes `true` once accumulated `tick`s reach the
    duration.
  - `progress()` on a `Transition::start(Duration::ZERO)` is `1.0`
    immediately (before any `tick`), and `is_complete()` is `true`
    immediately.
- [ ] Run `cargo test --lib transition::tests` — confirm compile
  failure.
- [ ] **Implement** `src/transition.rs`:
```rust
use std::time::Duration;

pub struct Transition {
    duration: Duration,
    elapsed: Duration,
}

impl Transition {
    pub fn start(duration: Duration) -> Self {
        Transition {
            duration,
            elapsed: Duration::ZERO,
        }
    }

    pub fn tick(&mut self, elapsed: Duration) {
        self.elapsed = (self.elapsed + elapsed).min(self.duration);
    }

    pub fn progress(&self) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        self.elapsed.as_secs_f32() / self.duration.as_secs_f32()
    }

    pub fn is_complete(&self) -> bool {
        self.elapsed >= self.duration
    }
}
```
- [ ] Add `pub mod transition;` to `src/lib.rs`.
- [ ] Run `cargo test --lib` — all green.
- [ ] `cargo fmt && cargo clippy --all-targets` — clean.
- [ ] Commit.

---

### Task 6: audio hook (no new dependency)

**Files:** new `src/audio.rs`; `src/lib.rs` (`pub mod audio;`).

**Interfaces produced:** `AudioSink` trait, `NullAudioSink`. See
spec's Task 6 section for the explicit no-`rodio` decision and
rationale — do not add `rodio` or any other dependency to `Cargo.toml`
as part of this task.

- [ ] **Write failing tests** in a new `#[cfg(test)] mod tests` in
  `src/audio.rs`:
  - `NullAudioSink::play(...)` does not panic (call it with an
    arbitrary `&str`, assert the test completes).
  - A test-local struct implementing `AudioSink` that records calls
    into a `Vec<String>` (or similar) confirms the trait is usable by
    an external implementor and that `play` is called with the exact
    `event_id` passed in — proves the trait shape is usable as a
    dependency-injection seam, not just that it compiles.
  - `Box<dyn AudioSink>` holding a `NullAudioSink` compiles and its
    `.play(...)` can be called — confirms the trait is object-safe
    (needed since apps will likely store this as a trait object).
- [ ] Run `cargo test --lib audio::tests` — confirm compile failure.
- [ ] **Implement** `src/audio.rs`:
```rust
pub trait AudioSink {
    fn play(&mut self, event_id: &str);
}

pub struct NullAudioSink;

impl AudioSink for NullAudioSink {
    fn play(&mut self, _event_id: &str) {}
}
```
- [ ] Add `pub mod audio;` to `src/lib.rs`.
- [ ] Run `cargo test --lib` — all green.
- [ ] `cargo fmt && cargo clippy --all-targets` — clean.
- [ ] Confirm `Cargo.toml` is unchanged (`git diff Cargo.toml` empty).
- [ ] Commit.

---

## Self-Review Notes

- **Spec coverage:** every design-doc section (Tasks 1-6) has exactly
  one corresponding plan task, with the exact signatures and rationale
  points (Task 4's forward-compatible `Cell` literal, Task 5's
  no-cross-dependency-on-Task-3 choice, Task 6's no-new-dependency
  decision) carried into the task text so an implementer doesn't need
  to read the spec separately.
- **Placeholder scan:** no TBDs; every task has literal signatures,
  literal implementation code, and literal test assertions with
  concrete values.
- **Independence check:** no task's implementation code imports
  another new-this-plan module — Task 4 imports only
  `crate::buffer`/`crossterm`; Task 5 deliberately does not import
  Task 3's `easing`. `src/lib.rs` is the only shared edit point, and
  it's additive (one line per task) — confirmed reconcilable without
  semantic conflict.
- **Type consistency:** `Cell`'s new `style: CellStyle` field (Task 1)
  is referenced by Task 4's `render()` via `..Default::default()`
  rather than a named field, so Task 4's code is correct whether or
  not Task 1 has landed yet in the branch it starts from.
