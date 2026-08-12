# Input Binding System — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-12
**Relationship to prior specs:** the third of the four future Arcs
named in the original post-windshield brainstorm (rendering depth/
perspective → the Falcon windshield + HUD Arcs, shipped/pending;
audio and data-viz widgets remain unbuilt). No code dependency on
those Arcs beyond consuming `src/app.rs`'s existing `App` trait
unchanged, and — for the adoption slice — extending
`examples/falcon/falcon.rs`'s existing `update`/`on_tick`, already
touched extensively by the HUD Arc.

## Problem

Every example app hand-rolls its own input handling: a raw
`match event.code { KeyCode::Tab => ..., KeyCode::Char('q') => ... }`,
duplicated with no shared vocabulary, no multi-key chord support, and
no reusable pattern for "requires N keys in sequence" or "requires a
specific modifier." This spec adds a general, framework-level
key-binding primitive — single keys and multi-key chords resolving to
an app-defined action type — and proves it with a real adoption in
Falcon, migrating its existing bindings and adding one new
chord-triggered action that couldn't exist without chord support.

## Scope

**`src/input.rs`: tag `coding`, TDD mandatory** — this is core
framework code, not example code, so the "Examples/demos" TDD
exception does not apply. The chord-resolution logic is pure state
(no I/O), fully unit-testable without a real terminal.

**Falcon adoption (`examples/falcon/falcon.rs`): tag `coding`,
TDD-exempt** per the "Examples/demos" exception, but
**`tools/visual-snapshot` is mandatory** for this slice — it changes
`on_tick`'s and `update`'s observable behavior and adds a new
render-triggering action (the `FullPower` flourish) that needs to
actually be seen firing correctly, not just reasoned about.

Three slices, in dependency order:

1. **`KeyPress` + `InputBinder<A>` core: `bind`/`feed`** (`src/input.rs`)
2. **`expire` (chord timeout)** (`src/input.rs`) — depends on 1.
3. **Falcon adoption** (`examples/falcon/falcon.rs`) — depends on 1-2.

## Design

### Slice 1-2: `KeyPress` + `InputBinder<A>`

```rust
//! Framework-level key-binding resolver: single keys and multi-key
//! chords resolve to an app-defined action type. Apps compose an
//! `InputBinder` into their own state (like `GlitchBuffer`/
//! `ParticleSystem`) rather than the `App` trait changing shape.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

/// A single key press to match against: code + modifiers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KeyPress {
    /// The key code (character, Tab, arrow, etc.).
    pub code: KeyCode,
    /// Required modifiers — matched via "actual contains required",
    /// not exact equality (see `InputBinder::feed`).
    pub modifiers: KeyModifiers,
}

impl KeyPress {
    /// A key press with no required modifiers — the common case.
    pub fn plain(code: KeyCode) -> Self {
        KeyPress {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl From<KeyPress> for Vec<KeyPress> {
    fn from(k: KeyPress) -> Self {
        vec![k]
    }
}

fn key_press_matches(actual: KeyPress, required: KeyPress) -> bool {
    actual.code == required.code && actual.modifiers.contains(required.modifiers)
}

fn sequence_starts_with(binding: &[KeyPress], seq: &[KeyPress]) -> bool {
    seq.len() <= binding.len()
        && seq
            .iter()
            .zip(binding.iter())
            .all(|(actual, required)| key_press_matches(*actual, *required))
}

fn binding_matches(binding: &[KeyPress], seq: &[KeyPress]) -> bool {
    binding.len() == seq.len() && sequence_starts_with(binding, seq)
}

/// Resolves key presses (single keys or multi-key chords) to an
/// app-defined action `A`. Compose into app state and feed it every
/// input event; call `expire` every tick to clear a stale in-progress
/// chord.
pub struct InputBinder<A: Copy> {
    bindings: Vec<(Vec<KeyPress>, A)>,
    pending: Vec<KeyPress>,
    pending_elapsed: Duration,
    chord_timeout: Duration,
}

impl<A: Copy> InputBinder<A> {
    /// `chord_timeout` bounds how long a partial chord waits for its
    /// next key before resetting.
    pub fn new(chord_timeout: Duration) -> Self {
        InputBinder {
            bindings: Vec::new(),
            pending: Vec::new(),
            pending_elapsed: Duration::ZERO,
            chord_timeout,
        }
    }

    /// Registers a binding — a single `KeyPress` (auto-converted to a
    /// length-1 sequence) or an explicit `Vec<KeyPress>` chord.
    pub fn bind(&mut self, sequence: impl Into<Vec<KeyPress>>, action: A) -> &mut Self {
        self.bindings.push((sequence.into(), action));
        self
    }

    /// Feeds one input event. Returns the resolved action once a full
    /// binding matches; `None` while a chord is still in progress or
    /// the event doesn't extend toward any binding. Ignores anything
    /// that isn't a `KeyEventKind::Press` key event.
    pub fn feed(&mut self, event: &Event) -> Option<A> {
        let Event::Key(key) = event else { return None };
        if key.kind != KeyEventKind::Press {
            return None;
        }
        let kp = KeyPress {
            code: key.code,
            modifiers: key.modifiers,
        };

        let mut candidate = self.pending.clone();
        candidate.push(kp);
        if let Some(action) = self.exact_match(&candidate) {
            self.pending.clear();
            self.pending_elapsed = Duration::ZERO;
            return Some(action);
        }
        if self.has_prefix_match(&candidate) {
            self.pending = candidate;
            self.pending_elapsed = Duration::ZERO;
            return None;
        }

        // Dead end for the extended sequence — retry fresh with just
        // this key, so an abandoned chord attempt doesn't swallow an
        // otherwise-valid single-key press.
        let fresh = vec![kp];
        if let Some(action) = self.exact_match(&fresh) {
            self.pending.clear();
            self.pending_elapsed = Duration::ZERO;
            return Some(action);
        }
        if self.has_prefix_match(&fresh) {
            self.pending = fresh;
        } else {
            self.pending.clear();
        }
        self.pending_elapsed = Duration::ZERO;
        None
    }

    /// Clears a pending chord once `chord_timeout` has elapsed since
    /// its last extending keypress. A no-op when nothing is pending.
    pub fn expire(&mut self, elapsed: Duration) {
        if self.pending.is_empty() {
            return;
        }
        self.pending_elapsed += elapsed;
        if self.pending_elapsed >= self.chord_timeout {
            self.pending.clear();
            self.pending_elapsed = Duration::ZERO;
        }
    }

    fn exact_match(&self, seq: &[KeyPress]) -> Option<A> {
        self.bindings
            .iter()
            .find(|(binding, _)| binding_matches(binding, seq))
            .map(|(_, action)| *action)
    }

    fn has_prefix_match(&self, seq: &[KeyPress]) -> bool {
        self.bindings
            .iter()
            .any(|(binding, _)| binding.len() > seq.len() && sequence_starts_with(binding, seq))
    }
}
```

Registered in `src/lib.rs` as `pub mod input;` (alphabetically between
`glitch` and `layout`), with a one-line `///` doc comment matching the
existing module list's style.

### Slice 3: Falcon adoption

```rust
use ttui::input::{InputBinder, KeyPress};

#[derive(Clone, Copy, PartialEq, Debug)]
enum FalconAction {
    FocusNext,
    FocusPrev,
    Whack,
    Quit,
    FullPower,
}

const CHORD_TIMEOUT: Duration = Duration::from_millis(1500);
const FULL_POWER_GLITCH_DURATION_MS: u64 = 500;

fn falcon_input() -> InputBinder<FalconAction> {
    let mut binder = InputBinder::new(CHORD_TIMEOUT);
    binder.bind(KeyPress::plain(KeyCode::Tab), FalconAction::FocusNext);
    binder.bind(KeyPress::plain(KeyCode::BackTab), FalconAction::FocusPrev);
    binder.bind(KeyPress::plain(KeyCode::Char(' ')), FalconAction::Whack);
    binder.bind(KeyPress::plain(KeyCode::Char('q')), FalconAction::Quit);
    binder.bind(
        vec![
            KeyPress::plain(KeyCode::Up),
            KeyPress::plain(KeyCode::Up),
            KeyPress::plain(KeyCode::Down),
            KeyPress::plain(KeyCode::Down),
        ],
        FalconAction::FullPower,
    );
    binder
}
```

`Falcon` gains an `input: InputBinder<FalconAction>` field, initialized
via `input: falcon_input(),` in `new()`.

`update()` replaces its raw `match k.code` with:

```rust
fn update(&mut self, event: &Event) {
    let Some(action) = self.input.feed(event) else { return };
    if action != FalconAction::Quit && self.booting.is_some() {
        return;
    }
    match action {
        FalconAction::Quit => self.quit = true,
        FalconAction::FocusNext => self.focused = (self.focused + 1) % PANELS.len(),
        FalconAction::FocusPrev => {
            self.focused = (self.focused + PANELS.len() - 1) % PANELS.len()
        }
        FalconAction::Whack => {
            if self.glitches[self.focused].is_active() {
                self.glitches[self.focused].clear();
                let (_, console) = Self::windshield_console_split(self.last_area.get());
                let slots = Self::panel_slots(console);
                let panel_box = Self::panel_box(slots[self.focused], true);
                self.spawn_whack_sparks(panel_box);
            }
        }
        FalconAction::FullPower => {
            let (_, console) = Self::windshield_console_split(self.last_area.get());
            let slots = Self::panel_slots(console);
            for (i, slot) in slots.iter().enumerate() {
                self.glitches[i].trigger(Duration::from_millis(FULL_POWER_GLITCH_DURATION_MS));
                let panel_box = Self::panel_box(*slot, i == self.focused);
                self.spawn_whack_sparks(panel_box);
            }
        }
    }
}
```

`FalconAction::Quit` stays reachable regardless of `booting` (matching
today's behavior — `q` already quits mid-boot); every other action is
blocked during boot, also matching today. `spawn_whack_sparks(panel_box:
Rect)` is a small new private helper extracting the existing
particle-spawn loop (currently inlined in the WHACK match arm) so
`FullPower` can reuse it per-panel without duplicating the loop body —
purely a mechanical extraction, no behavior change to the existing
WHACK spark burst.

`on_tick()` gains, right after the existing `self.tick_count += 1;`:

```rust
self.input.expire(elapsed);
```

## Non-goals

- **Mouse support.** Explicitly deferred — see the scope discussion in
  the design conversation. No terminal mouse-capture change, no
  `Event::Mouse` handling anywhere in this Arc.
- **Runtime-configurable/rebindable bindings.** `InputBinder`'s
  bindings are fixed at construction (via `bind` calls in the app's own
  setup code) — no in-app UI for rebinding, no config-file loading.
- **Changing the `App` trait.** `InputBinder` is a composed utility
  type, not a new trait requirement — apps that don't need it are
  unaffected.
- **Wildcard/"any key" chord entries.** Every position in a chord is a
  concrete `KeyPress`; there's no "match any key here" placeholder.
- **A built-in bindings/help display.** No auto-generated cheatsheet or
  on-screen hint listing registered bindings.
- **Cross-boot chord state is not specially handled.** A chord could
  theoretically start being typed during Falcon's ~1.4s boot sequence
  and complete just after boot ends. Given the timeout (1.5s) already
  bounds how long partial state survives and boot is short, this is
  treated as an acceptable non-issue, not engineered around.

## Testing

Per `.claude/rules/development-conventions.md`, `src/input.rs` is
`coding`-tagged with no TDD exception — tests are written first, per
task, in the implementation plan. Concrete behaviors the test suite
must cover:

- A single-key binding fires on its first matching press.
- An unmatched key returns `None` without panicking.
- A multi-key chord requires all its keys, in order, before firing.
- A partial chord (fewer keys than the binding) does not fire early.
- An abandoned chord prefix falls through to a valid single-key
  binding on the very key that broke the chord (the "doesn't swallow a
  valid keypress" behavior).
- A key that is a dead end (matches no binding and no prefix) clears
  `pending` with no fired action.
- `expire` clears a stale pending chord once `chord_timeout` has
  elapsed.
- `expire` does nothing when nothing is pending.
- `expire` does *not* clear a chord that's still within its timeout
  window.
- Modifier matching uses "actual contains required," not exact
  equality — a binding registered with `KeyModifiers::NONE` fires
  regardless of extra modifier bits on the actual event; a binding
  requiring `KeyModifiers::CONTROL` does not fire without it.
- `KeyEventKind::Release`/`Repeat` events are ignored by `feed`.
- Non-`Event::Key` events (resize, mouse, paste, focus) are ignored.
- Two registered chords sharing a common prefix (e.g. `g g` and `g h`)
  each resolve correctly and don't cross-fire.

Falcon's adoption slice: TDD-exempt, verified via
`tools/visual-snapshot` — capture Tab/Shift+Tab cycling (unchanged
focus behavior), Space-while-glitching (unchanged WHACK spark burst),
and the new Up-Up-Down-Down chord (confirming all three panels'
glitches fire simultaneously and a particle burst appears at each
panel's center).

## Critical files

- `src/input.rs` — new module: `KeyPress`, `InputBinder<A>`.
- `src/lib.rs` — adds `pub mod input;`.
- `examples/falcon/falcon.rs` — `FalconAction`, `falcon_input()`,
  `update`/`on_tick` changes, new `spawn_whack_sparks` helper.

## Verification

- `cargo build --all-targets` / `cargo clippy --all-targets -- -D
  warnings` / `cargo fmt --check` — clean.
- `cargo test` — all new `src/input.rs` unit tests green, full existing
  suite unchanged elsewhere.
- `tools/visual-snapshot` capture of Falcon confirming Tab/Shift+Tab
  and Space-while-glitching behave exactly as before, plus a new
  capture of the Up-Up-Down-Down chord firing the `FullPower` flourish
  (all three panels glitching + particle bursts simultaneously).
