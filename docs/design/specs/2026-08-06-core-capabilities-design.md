# Shared Core Capabilities (Arc 0) — Design

**Status:** draft, self-authored during autonomous execution of the
example-apps roadmap (see "Note on process" below) — pending your
review.
**Date:** 2026-08-06
**Relationship to prior specs:** implements Arc 0 of
`2026-08-06-example-apps-roadmap-design.md` (issues #34-#39, tracking
#40) — the six capabilities needed by 2+ of the Omnitrix/Smash
Crabs/TARDIS example arcs. Builds on the unchanged, fully-implemented
Rev A (`2026-08-04-ttui-core-framework-design.md`) and Rev B
(`2026-08-05-ttui-rev-b-vision-alignment-design.md`) specs, and the
shipped `LayerStack`/`composite()` work
(`2026-08-05-buffer-layering-compositing-design.md`).

## Note on process

You asked me to run subagent-driven development on Arc 0 concurrently
and only surface on completion or a breaking finding. Per
`.claude/rules/development-conventions.md`, each of these six tickets
needs its own brainstorm -> spec -> plan cycle before code lands —
normally an interactive dialogue. Running that dialogue six times over
would contradict "manage this without checking in," so I made the
design calls myself here, grounded in the roadmap spec's stated intent
for each ticket and this codebase's existing patterns (documented
per-section below). Flagging this plainly rather than quietly treating
six brainstorms as skippable: if any of these six calls look wrong
once you see the code, that's exactly the kind of thing to redirect.

## Scope of this spec

Six capabilities, each independent of the others (no ticket in this
arc depends on another — confirmed below), each landing as one new
file plus a one-line `pub mod` addition to `src/lib.rs` (the only
point of file overlap between tasks — trivial to reconcile since
`LayerStack`/`Buffer`'s own module already coexists there), except
Task 1 (`Cell` style), which touches existing widget/example call
sites as an unavoidable consequence of adding a field to a struct
several files already construct exhaustively.

**Explicitly not designed here:** how Omnitrix/Smash Crabs/TARDIS
*use* these primitives (that's each example arc's own Structural/
Architectural/Features+Polish waves, each still gated on its own
future brainstorm per the roadmap plan).

## Design

### Task 1: `Cell` style (bold)

`Cell` (`src/buffer.rs`) gains a fourth field:

```rust
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CellStyle {
    pub bold: bool,
}
```

added to `Cell` as `pub style: CellStyle`, included in `Cell::default()`.
Nine existing exhaustive `Cell { symbol, fg, bg }` literals across
`src/widgets/{text,block,list,table}.rs` and two in
`examples/smash_crabs.rs` need `, ..Default::default()` appended (or
`, style: CellStyle::default()`) — Rust's exhaustive-literal rule
means this is not optional, not a design choice.

`src/terminal.rs::draw_diff` gains attribute wiring: emit
`SetAttribute(Attribute::Reset)` then, if `d.cell.style.bold`,
`SetAttribute(Attribute::Bold)`, before the existing `Print` — matching
the file's existing per-cell, fully-stateless write pattern (every cell
already re-sets fg/bg unconditionally; attributes follow the same
rule, no cross-cell state to track). This part of `terminal.rs` is
real-TTY-dependent per `.claude/rules/development-conventions.md`'s
TDD exception — verified by `cargo run --example` visually, not a unit
test, matching every other `Terminal` method already in that file.

**Why bold-only, not a broader attribute set:** the roadmap ticket says
"bold at minimum," both consuming vision-doc passages (Omnitrix glow,
Smash Crabs loud text) only need bold, and YAGNI argues against
speculatively adding underline/dim now with no consumer.

### Task 2: screen-shake helper

New file `src/effects.rs`:

```rust
pub fn shake(buf: &Buffer, dx: i16, dy: i16) -> Buffer
```

Returns a new `Buffer` with every cell shifted by `(dx, dy)`; cells
shifted in from outside the original bounds are `Cell::default()`.
Pure, stateless, no RNG — the roadmap ticket's "decaying random
offset... for N ticks" splits at the same app-space boundary Rev B
already drew for camera/decay: core provides the shift transform,
app-space owns the tick-driven random-offset sequence (also avoids
adding an RNG dependency, preserving Rev A's single-dependency
posture).

### Task 3: easing/tween helper

New file `src/easing.rs`:

```rust
pub fn lerp(start: f32, end: f32, t: f32) -> f32
pub fn ease_out(start: f32, end: f32, t: f32) -> f32
pub fn progress(elapsed: Duration, duration: Duration) -> f32
```

`lerp`/`ease_out` clamp `t` to `[0, 1]` before interpolating.
`ease_out` uses `1 - (1-t)^2` (quadratic ease-out) as the eased `t`
fed into `lerp`. `progress` returns `elapsed / duration` clamped to
`[0, 1]`, and `1.0` when `duration` is zero (avoids division by zero;
"no duration to wait out" reads as "already there").

### Task 4: particle system

New file `src/particles.rs`:

```rust
pub struct Particle {
    pub x: f32, pub y: f32,
    pub vx: f32, pub vy: f32,
    pub symbol: char,
    pub color: Color,
    pub lifetime: Duration,
    pub age: Duration,
}
pub struct ParticleSystem { /* private Vec<Particle> */ }

impl ParticleSystem {
    pub fn new() -> Self
    pub fn spawn(&mut self, p: Particle)
    pub fn update(&mut self, elapsed: Duration)  // advances position by velocity*elapsed, ages particles, drops expired ones (age >= lifetime)
    pub fn render(&self, buf: &mut Buffer)        // writes each alive particle's symbol/color at its rounded (x, y), skipping any outside buf's bounds
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
}
```

`render`'s `Cell` literal must use `, ..Default::default()` (not a
bare 3-field literal) so this file compiles unchanged whether it lands
before or after Task 1's `style` field, regardless of merge order
between these two file-disjoint, genuinely-parallel tasks.

### Task 5: transition hook

New file `src/transition.rs`:

```rust
pub struct Transition { /* private duration: Duration, elapsed: Duration */ }

impl Transition {
    pub fn start(duration: Duration) -> Self   // elapsed = 0
    pub fn tick(&mut self, elapsed: Duration)  // elapsed += elapsed, clamped to duration (never overshoots)
    pub fn progress(&self) -> f32              // elapsed / duration, in [0, 1]; 1.0 if duration is zero
    pub fn is_complete(&self) -> bool          // elapsed >= duration
}
```

**Why a plain state struct, not new `App` trait methods:** the ticket
frames this as "so app-switch effects share one mechanism instead of
three bespoke ones," but nothing about counting elapsed transition time
and computing a 0..1 progress actually requires framework-loop
involvement — `on_tick`/`tick_rate` (Rev B) already deliver elapsed
time to app code every tick. `Transition` is the reusable piece (the
boilerplate three examples would otherwise each reinvent), used as
ordinary app state exactly like `Theme` or `Focus` already are — this
matches Rev B's "Not a gap" precedent (focus/navigation is app state,
not framework machinery) and the buffer-layering spec's app-space
boundary (App only exposes structure; apps assign meaning). It also
means this task touches zero existing files (no `App` trait change),
keeping it genuinely parallel with the other five.

Deliberately **not** built on Task 3's `easing::progress` despite
nearly identical math (both are `elapsed/duration` clamped to `[0,1]`):
the roadmap spec scoped these six tickets as independent, and a
same-arc cross-dependency here would force sequential landing for two
lines of duplicated arithmetic — not worth trading away real
concurrency for.

### Task 6: audio hook — explicit dependency decision

The roadmap ticket explicitly asks this brainstorm to "reach an
explicit decision, not assume yes" on adding `rodio`. Decision: **no
new dependency lands in `ttui` core.** Rev A states the single-
dependency (`crossterm`-only) posture as a deliberate, repeated
commitment, and `rodio` pulls in a non-trivial dependency tree (`cpal`,
codec backends) for a capability only two of three vision docs mark
"Optional"/"Highly Recommended," not required. Instead, core exposes a
seam; a concrete backend (rodio or otherwise) is an *app's* dependency,
added only when Smash Crabs' or TARDIS's audio-cue tickets (#66, #81)
actually pick it up.

New file `src/audio.rs`:

```rust
pub trait AudioSink {
    fn play(&mut self, event_id: &str);
}

pub struct NullAudioSink;
impl AudioSink for NullAudioSink {
    fn play(&mut self, _event_id: &str) {}
}
```

`NullAudioSink` is the zero-cost default for apps that don't wire
audio at all — mirrors `tick_rate() -> Option<Duration> { None }`'s
opt-in-with-a-free-default shape from Rev B.

## Testing

All six are `coding`-tagged, TDD-mandatory, no exceptions apply (none
are config/git-adjacent, examples, real-TTY-only, or research spikes)
— except Task 1's `terminal.rs` half, which is real-TTY-dependent per
the exception in `.claude/rules/development-conventions.md` and is
verified manually via `cargo run --example`, same as every other
`Terminal` method. Test cases per task are enumerated in the
implementation plan (`2026-08-06-core-capabilities-plan.md`).

## Concurrency note

Per `superpowers:subagent-driven-development`'s default ("never
dispatch multiple implementation subagents in parallel — conflicts"):
this arc deliberately deviates. All six tasks are file-disjoint except
for a one-line `pub mod` addition each makes to `src/lib.rs` (six
independent one-liners — trivial to reconcile by hand at integration,
not a real conflict). Each task is dispatched to a worktree-isolated
implementer so genuine concurrent mutation is safe, per this session's
explicit instruction to manage Arc 0 as concurrent development.

## Critical files

- `src/buffer.rs` — `CellStyle`, `Cell.style` (Task 1 only).
- `src/terminal.rs` — bold attribute wiring in `draw_diff` (Task 1 only).
- `src/widgets/{text,block,list,table}.rs`, `examples/smash_crabs.rs`
  — literal-site updates for the new `Cell` field (Task 1 only).
- `src/effects.rs` — new (Task 2).
- `src/easing.rs` — new (Task 3).
- `src/particles.rs` — new (Task 4).
- `src/transition.rs` — new (Task 5).
- `src/audio.rs` — new (Task 6).
- `src/lib.rs` — one `pub mod` line per new file (Tasks 2-6).

## Verification

- `cargo test` green after each task lands and after final integration.
- `cargo fmt` / `cargo clippy --all-targets` clean.
- `cargo build --examples` — all three existing examples still compile
  after Task 1's `Cell` field addition.
- `cargo run --example omnitrix` / `smash_crabs` — manual visual check
  that Task 1 introduces no regression (bold wiring is additive; no
  example uses it yet).
