# Input Binding System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a general, framework-level key-binding primitive (`src/input.rs`) supporting single keys and multi-key chords, and prove it with a real adoption in Falcon that migrates its existing bindings and adds one new chord-only action.

**Architecture:** `InputBinder<A>` is a composed utility type (like `GlitchBuffer`/`ParticleSystem`) an app holds as a field, not a change to the `App` trait. `feed(&mut self, event) -> Option<A>` resolves one key event against registered bindings (chords are just length-N sequences); `expire(&mut self, elapsed)` clears a stale in-progress chord after a timeout.

**Tech Stack:** Rust, `crossterm::event` (`Event`, `KeyCode`, `KeyEventKind`, `KeyModifiers`).

## Global Constraints

- **`src/input.rs` is `coding`-tagged with TDD mandatory** — no exemption. Every behavior below is tested via a failing test written first.
- **The Falcon adoption slice (Task 3) is TDD-exempt** (example code, "Examples/demos" exception in `.claude/rules/development-conventions.md`), but **`tools/visual-snapshot` is mandatory** for it — it changes `on_tick`'s and `update`'s observable behavior and adds a new render-triggering action.
- **Modifier matching is "actual contains required," not exact equality** — a binding registered with `KeyModifiers::NONE` matches regardless of what other modifier bits are set on the real event. This preserves every existing example's current behavior of never checking modifiers.
- **No mouse support, no runtime-rebindable bindings, no `App` trait change, no wildcard chord entries, no built-in bindings/help display** — see the spec's Non-goals for full rationale.

---

### Task 1: `KeyPress` + `InputBinder` core (`bind`/`feed`)

**Files:**
- Create: `src/input.rs`
- Modify: `src/lib.rs` (add `pub mod input;`)

**Interfaces:**
- Produces: `pub struct KeyPress { pub code: KeyCode, pub modifiers: KeyModifiers }`, `impl KeyPress { pub fn plain(code: KeyCode) -> Self }`, `impl From<KeyPress> for Vec<KeyPress>`, `pub struct InputBinder<A: Copy>`, `impl<A: Copy> InputBinder<A> { pub fn new(chord_timeout: Duration) -> Self; pub fn bind(&mut self, sequence: impl Into<Vec<KeyPress>>, action: A) -> &mut Self; pub fn feed(&mut self, event: &Event) -> Option<A>; }` — all consumed by Task 2 (adds `expire`) and Task 3 (Falcon adoption).

- [ ] **Step 1: Write the failing tests**

Create `src/input.rs` with just enough scaffolding for the test module to compile against (the real types don't exist yet, so this step's tests will fail to compile — that's the expected "red" for brand-new Rust code):

```rust
//! Framework-level key-binding resolver: single keys and multi-key
//! chords resolve to an app-defined action type. Apps compose an
//! `InputBinder` into their own state (like `GlitchBuffer`/
//! `ParticleSystem`) rather than the `App` trait changing shape.

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use std::time::Duration;

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn press_with(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum TestAction {
        A,
        B,
        Chord,
    }

    #[test]
    fn single_key_binding_fires_on_first_matching_press() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        assert_eq!(binder.feed(&press(KeyCode::Char('a'))), Some(TestAction::A));
    }

    #[test]
    fn unmatched_key_returns_none() {
        let mut binder = InputBinder::<TestAction>::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        assert_eq!(binder.feed(&press(KeyCode::Char('z'))), None);
    }

    #[test]
    fn non_key_events_are_ignored() {
        let mut binder = InputBinder::<TestAction>::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        assert_eq!(binder.feed(&Event::Resize(80, 24)), None);
    }

    #[test]
    fn key_release_and_repeat_events_are_ignored() {
        use crossterm::event::KeyEventKind;
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(KeyPress::plain(KeyCode::Char('a')), TestAction::A);
        let release = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        assert_eq!(binder.feed(&release), None);
        let repeat = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        ));
        assert_eq!(binder.feed(&repeat), None);
    }

    #[test]
    fn two_key_chord_requires_both_keys_in_order() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![KeyPress::plain(KeyCode::Char('g')), KeyPress::plain(KeyCode::Char('g'))],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None, "partial chord doesn't fire early");
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), Some(TestAction::Chord));
    }

    #[test]
    fn partial_chord_does_not_fire_early() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![
                KeyPress::plain(KeyCode::Up),
                KeyPress::plain(KeyCode::Up),
                KeyPress::plain(KeyCode::Down),
            ],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Up)), None);
        assert_eq!(binder.feed(&press(KeyCode::Up)), None, "still only 2 of 3 keys");
    }

    #[test]
    fn abandoned_chord_prefix_falls_through_to_a_valid_single_key_binding() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![KeyPress::plain(KeyCode::Char('g')), KeyPress::plain(KeyCode::Char('g'))],
            TestAction::Chord,
        );
        binder.bind(KeyPress::plain(KeyCode::Char('q')), TestAction::A);
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None, "starts the gg chord");
        assert_eq!(
            binder.feed(&press(KeyCode::Char('q'))),
            Some(TestAction::A),
            "q breaks the chord but still fires its own binding, not swallowed"
        );
    }

    #[test]
    fn dead_end_key_clears_pending_with_no_action() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![KeyPress::plain(KeyCode::Char('g')), KeyPress::plain(KeyCode::Char('g'))],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None, "starts the gg chord");
        assert_eq!(
            binder.feed(&press(KeyCode::Char('z'))),
            None,
            "z matches nothing, no prefix — dead end, no action fires"
        );
        // Pending was cleared, so a fresh gg still works afterward:
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), Some(TestAction::Chord));
    }

    #[test]
    fn overlapping_prefix_chords_resolve_to_the_correct_one() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        binder.bind(
            vec![KeyPress::plain(KeyCode::Char('g')), KeyPress::plain(KeyCode::Char('g'))],
            TestAction::A,
        );
        binder.bind(
            vec![KeyPress::plain(KeyCode::Char('g')), KeyPress::plain(KeyCode::Char('h'))],
            TestAction::B,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        assert_eq!(binder.feed(&press(KeyCode::Char('h'))), Some(TestAction::B));

        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), Some(TestAction::A));
    }

    #[test]
    fn modifier_matching_uses_contains_not_exact_equality() {
        let mut binder = InputBinder::new(Duration::from_secs(1));
        // Registered with NONE — should fire even if SHIFT happens to be set.
        binder.bind(KeyPress::plain(KeyCode::BackTab), TestAction::A);
        assert_eq!(
            binder.feed(&press_with(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(TestAction::A)
        );

        // Registered requiring CONTROL — must not fire without it.
        let mut ctrl_binder = InputBinder::new(Duration::from_secs(1));
        ctrl_binder.bind(
            KeyPress { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL },
            TestAction::A,
        );
        assert_eq!(ctrl_binder.feed(&press(KeyCode::Char('c'))), None);
        assert_eq!(
            ctrl_binder.feed(&press_with(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(TestAction::A)
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib input:: -- --nocapture`
Expected: FAIL to compile — `KeyPress`/`InputBinder` don't exist yet (`cannot find type` errors). This is the expected red state for a brand-new Rust module.

- [ ] **Step 3: Write the implementation**

Add this above the `#[cfg(test)]` block in `src/input.rs`:

```rust
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

In `src/lib.rs`, add (alphabetically between the `glitch` and `layout` module declarations):

```rust
/// Key-binding resolver: single keys and multi-key chords resolving
/// to an app-defined action type.
pub mod input;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib input::`
Expected: all tests in `src/input.rs` PASS.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

Run: `cargo fmt --check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/input.rs src/lib.rs
git commit -m "feat(core): add InputBinder key/chord resolver

A framework-level key-binding primitive supporting single keys and
multi-key chords, composed into app state like GlitchBuffer/
ParticleSystem rather than changing the App trait."
```

---

### Task 2: `expire` (chord timeout)

**Files:**
- Modify: `src/input.rs`

**Interfaces:**
- Consumes: `InputBinder<A>`'s private `pending`/`pending_elapsed`/`chord_timeout` fields (Task 1, same file/impl block).
- Produces: `pub fn expire(&mut self, elapsed: Duration)` — consumed by Task 3 (Falcon's `on_tick`).

- [ ] **Step 1: Write the failing tests**

Add to `src/input.rs`'s `#[cfg(test)] mod tests` block (inside the existing `mod tests`, alongside Task 1's tests):

```rust
    #[test]
    fn expire_does_nothing_when_pending_is_empty() {
        let mut binder = InputBinder::<TestAction>::new(Duration::from_millis(100));
        binder.expire(Duration::from_secs(10)); // no panic, no effect
        binder.bind(
            vec![KeyPress::plain(KeyCode::Char('g')), KeyPress::plain(KeyCode::Char('g'))],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), Some(TestAction::Chord));
    }

    #[test]
    fn expire_clears_a_stale_pending_chord_after_timeout() {
        let mut binder = InputBinder::new(Duration::from_millis(100));
        binder.bind(
            vec![KeyPress::plain(KeyCode::Char('g')), KeyPress::plain(KeyCode::Char('g'))],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None, "starts the chord");
        binder.expire(Duration::from_millis(150)); // past the 100ms timeout
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            None,
            "pending was cleared by the timeout, so this is a fresh first key, not the chord's second"
        );
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord),
            "a fresh gg from here still completes normally"
        );
    }

    #[test]
    fn expire_does_not_clear_a_chord_still_within_timeout() {
        let mut binder = InputBinder::new(Duration::from_millis(100));
        binder.bind(
            vec![KeyPress::plain(KeyCode::Char('g')), KeyPress::plain(KeyCode::Char('g'))],
            TestAction::Chord,
        );
        assert_eq!(binder.feed(&press(KeyCode::Char('g'))), None);
        binder.expire(Duration::from_millis(50)); // under the 100ms timeout
        assert_eq!(
            binder.feed(&press(KeyCode::Char('g'))),
            Some(TestAction::Chord),
            "chord still completes — the first key's pending state survived"
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib input::`
Expected: FAIL to compile — `expire` doesn't exist yet on `InputBinder`.

- [ ] **Step 3: Write the implementation**

In `src/input.rs`, add this method to `impl<A: Copy> InputBinder<A>` (after `feed`, before the private helpers):

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib input::`
Expected: all tests PASS (Task 1's and Task 2's together).

- [ ] **Step 5: Lint, format, full suite**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.
Run: `cargo test` — full suite green (207 + N new `input` tests + 14 launcher tests, 0 failed).

- [ ] **Step 6: Commit**

```bash
git add src/input.rs
git commit -m "feat(core): add InputBinder chord-timeout expiry

expire() clears a stale in-progress chord after chord_timeout elapses,
called from an app's on_tick — without it, a chord started and never
finished would wait forever for its remaining keys."
```

---

### Task 3: Falcon adoption

**Files:**
- Modify: `examples/falcon/falcon.rs`

**Interfaces:**
- Consumes: `ttui::input::{InputBinder, KeyPress}` (Tasks 1-2, exact signatures above). Existing `Falcon` fields/methods: `glitches: [GlitchBuffer; 3]`, `particles: ParticleSystem`, `theme: Theme`, `last_area: std::cell::Cell<Rect>`, `windshield_console_split`, `panel_slots`, `panel_box`, `PANELS`, `WHACK_SPARK_COUNT`, `WHACK_SPARK_LIFETIME_MS` (all already exist in `examples/falcon/falcon.rs`/`examples/falcon/hud.rs` as of the merged HUD Arc).
- Produces: `FalconAction` enum, `falcon_input() -> InputBinder<FalconAction>`, `Falcon::spawn_whack_sparks(&mut self, panel_box: Rect)` — none consumed by later tasks in this plan (Task 4 is verification-only).

- [ ] **Step 1: Add the import, action enum, and constants**

At the top of `examples/falcon/falcon.rs`, add to the existing `use ttui::...` imports:

```rust
use ttui::input::{InputBinder, KeyPress};
```

Near the top of the file, after the existing const block (right after `const WEAPONS_PULSE_SPEED: f32 = 3.0;` or wherever the last const in `falcon.rs` currently sits — locate by content, not line number, since Tasks in the HUD Arc already shifted these), add:

```rust
const CHORD_TIMEOUT: Duration = Duration::from_millis(1500);
const FULL_POWER_GLITCH_DURATION_MS: u64 = 500;
```

Add this enum near `PanelKind` (same general area of the file):

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
enum FalconAction {
    FocusNext,
    FocusPrev,
    Whack,
    Quit,
    FullPower,
}
```

- [ ] **Step 2: Add `falcon_input()`**

Add this function near `falcon_camera()`:

```rust
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

- [ ] **Step 3: Add the `input` field**

In the `Falcon` struct, add a new field (anywhere in the struct body — e.g. right after `theme: Theme,`):

```rust
    input: InputBinder<FalconAction>,
```

In `Falcon::new()`, add the matching initializer (e.g. right after `theme: falcon_theme(),`):

```rust
            input: falcon_input(),
```

- [ ] **Step 4: Extract `spawn_whack_sparks`**

Add this new private method to `impl Falcon` (a mechanical extraction of the particle-spawn loop that currently lives inline in the WHACK key handler):

```rust
    fn spawn_whack_sparks(&mut self, panel_box: Rect) {
        let cx = panel_box.x as f32 + panel_box.width as f32 / 2.0;
        let cy = panel_box.y as f32 + panel_box.height as f32 / 2.0;
        for i in 0..WHACK_SPARK_COUNT {
            let angle = i as f32 * std::f32::consts::TAU / WHACK_SPARK_COUNT as f32;
            self.particles.spawn(Particle {
                x: cx,
                y: cy,
                vx: angle.cos() * 6.0,
                vy: angle.sin() * 3.0,
                symbol: '*',
                color: self.theme.accent,
                lifetime: Duration::from_millis(WHACK_SPARK_LIFETIME_MS),
                age: Duration::ZERO,
            });
        }
    }
```

- [ ] **Step 5: Rewrite `update()`**

Replace the entire body of `update()` (currently a raw `match k.code` reached via `let Event::Key(k) = event else { return }; if k.kind != KeyEventKind::Press { return; } if k.code == KeyCode::Char('q') { ... } if self.booting.is_some() { return; } match k.code { ... }`) with:

```rust
    fn update(&mut self, event: &Event) {
        let Some(action) = self.input.feed(event) else {
            return;
        };
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
                    self.glitches[i]
                        .trigger(Duration::from_millis(FULL_POWER_GLITCH_DURATION_MS));
                    let panel_box = Self::panel_box(*slot, i == self.focused);
                    self.spawn_whack_sparks(panel_box);
                }
            }
        }
    }
```

`crossterm::event::{KeyCode, KeyEventKind}` may now be unused imports in `falcon.rs` if nothing else in the file references them directly — check with `cargo build --example falcon` after this step; if either import is now unused, remove it (KeyCode is very likely still needed for `falcon_input()`'s `KeyPress::plain(KeyCode::Tab)` etc., so this is probably a non-issue, but `KeyEventKind` may no longer be referenced anywhere in `falcon.rs` now that the manual `k.kind != KeyEventKind::Press` check moved inside `InputBinder::feed` — remove that import if the compiler flags it unused).

- [ ] **Step 6: Add `self.input.expire(elapsed)` to `on_tick`**

In `on_tick()`, right after the existing `self.tick_count += 1;` line, add:

```rust
        self.input.expire(elapsed);
```

- [ ] **Step 7: Build, lint, format**

Run: `cargo build --all-targets` — succeeds, no warnings.
Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 8: Capture and verify visually — unchanged behavior**

Using `tools/visual-snapshot`, capture a post-boot script exercising Tab, Shift+Tab, and Space while a panel is glitching (e.g. wait for boot, wait for an idle-flicker glitch to trigger on the focused panel — or trigger one deterministically by waiting through a full `IDLE_FLICKER_PERIOD_TICKS` cycle — then press Space). `Read` the resulting frames. Confirm: focus cycling and the WHACK spark burst look exactly as they did before this task (same visual behavior, since the migration is meant to be behavior-preserving).

- [ ] **Step 9: Capture and verify visually — FullPower chord**

Capture a post-boot script that sends `Up`, `Up`, `Down`, `Down` in sequence with short waits between each (well under `CHORD_TIMEOUT` = 1500ms), then a short wait after the last key. `Read` the resulting frames. Confirm: all three console panels show a glitch burst simultaneously, and a particle burst (small `*` sparks) appears at each of the three panels' centers at the same time — not just the focused one.

- [ ] **Step 10: Commit**

```bash
git add examples/falcon/falcon.rs
git commit -m "feat(falcon): adopt InputBinder, add Up-Up-Down-Down FullPower chord

Migrates Tab/Shift+Tab/Space/q onto the new framework-level binding
resolver (behavior-preserving — WHACK's glitch-active gate moves from
a match guard to a plain check after resolution, same effect) and adds
one new chord-only action that couldn't exist without chord support."
```

---

### Task 4: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: full suite green — includes all new `src/input.rs` tests from Tasks 1-2; no other test file changes in this plan, so everything else is unchanged from before this Arc.

- [ ] **Step 4: One more full `tools/visual-snapshot` capture of the finished result**

Run a capture spanning: boot, Tab cycling through all three panels (confirming the HUD Arc's per-focus HUD states — Hyperdrive/Sensors/Weapons — still swap correctly, since this Arc's `update()` rewrite could plausibly have broken that if focus-cycling regressed), a WHACK spark burst, and the Up-Up-Down-Down FullPower chord. `Read` it. This is the final, whole-Arc confirmation that the migration didn't regress anything from the HUD Arc while adding the new chord capability. Reference this capture in the PR's Verification section.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo test` — full suite green, including all new `src/input.rs` tests.
- [ ] At least one `tools/visual-snapshot` capture from Task 4 is referenced in the PR description, showing unchanged Falcon behavior plus the new FullPower chord.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main` — **this branch is stacked on the still-unmerged `worktree-falcon-hud-system` branch (PR #102)**, so the PR's base should be `worktree-falcon-hud-system`, not `main`, and the PR template's "Stacked PR note" section should say so explicitly (`Base PR: #102`, not safe to review independently until #102 merges). Wait for the four required checks green, squash-merge (after #102 has merged and this PR has been re-based/retargeted to `main` if needed), then remove the worktree via `ExitWorktree` (per the documented squash-merge resolution: verify via `gh pr view --json state,mergedAt,mergeCommit`, then retry with `discard_changes: true` if the tool's own ancestry check false-positives).
