# Launcher Visual Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the launcher's nexus a real drifting starfield, real enlarge-on-focus, a dive-in transition flourish, and gradient portal borders — closing two gaps between the original launcher spec and what shipped, building the previously-optional dive flourish, and applying Arc A's gradient-border capability.

**Architecture:** All changes are confined to `examples/launcher/{main,nexus,portal}.rs`. `Launcher` gains two new persistent fields (`starfield: ParticleSystem`, `diving: Option<(usize, Transition, ParticleSystem)>`); `nexus::render` gains a `&ParticleSystem` parameter; `portals()`'s sizing becomes per-portal instead of uniform; `portal::draw`'s `Theme` gains a conditional `primary_end`. No `src/` changes.

**Tech Stack:** Rust, existing `ttui` core (`particles`, `transition`, `camera`, `theme`).

## Global Constraints

- **Tag: `coding`.** TDD applies to `Launcher`'s state machine (already unit-tested today) — new/changed state gets tests first. Pixel/visual output (`nexus`/`portal` rendering) stays the existing examples carve-out: verified by running, not asserted on.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are hard gates on every task.
- `App::on_tick`/`Launcher::apply` do not have access to the terminal's real size (only `view` does) — every task below that needs a position/area works around this with a fixed nominal/virtual space, explicitly, not by threading new state through the `App` trait.
- One worktree for this whole Arc, created via `superpowers:using-git-worktrees` before Task 1, per `.claude/rules/git-github-standards.md`.
- `coding`-tagged → **Gated** autonomy tier: ships as a PR to `main` with all four required checks green, squash-merged at the end.
- Spec being implemented: `docs/design/specs/launcher/2026-08-09-launcher-visual-polish-design.md`.

---

### Task 1: Starfield — static hash-grid → real `ParticleSystem` drift

**Files:**
- Modify: `examples/launcher/main.rs`
- Modify: `examples/launcher/nexus.rs`

**Interfaces:**
- Consumes: `ttui::particles::{Particle, ParticleSystem}` (existing, unchanged).
- Produces: `Launcher.starfield: ParticleSystem` field, `nexus::render`'s new `&ParticleSystem` parameter — Tasks 2-4 build on this same `render` signature.

- [ ] **Step 1: Write the failing test**

In `examples/launcher/main.rs`'s `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn starfield_tops_up_to_target_count_after_ticking() {
        let mut l = Launcher::new();
        assert_eq!(l.starfield.len(), 0);
        for _ in 0..TARGET_STAR_COUNT {
            l.on_tick(Duration::from_millis(50));
        }
        assert_eq!(l.starfield.len(), TARGET_STAR_COUNT);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib -p ttui --test '*' 2>/dev/null; cargo build --example launcher`
Expected: FAIL to compile — `Launcher` has no `starfield` field yet, `TARGET_STAR_COUNT` doesn't exist.

- [ ] **Step 3: Add starfield state and spawn logic to `main.rs`**

Add near the top of the file, alongside the existing `NEXUS_TICK`/`RETURN_FADE_MS` constants:

```rust
const STARFIELD_W: u16 = 250;
const STARFIELD_H: u16 = 80;
const TARGET_STAR_COUNT: usize = 60;
const STAR_LIFETIME_SECS: u64 = 30;
```

Add to the imports: change `use ttui::transition::Transition;` to also bring in particles:

```rust
use ttui::particles::{Particle, ParticleSystem};
use ttui::transition::Transition;
```

Add this free function (near `dim_color`/`text_center`):

```rust
/// Spawns one drifting background star at a pseudo-random position and
/// velocity within the fixed virtual starfield space, derived from
/// `seed` (a monotonically-increasing counter, not real randomness —
/// deterministic and dependency-free, matching this codebase's
/// existing hash-based pseudo-random patterns).
fn spawn_star(seed: u64) -> Particle {
    let h1 = seed.wrapping_mul(2_654_435_761);
    let h2 = seed.wrapping_mul(2_246_822_519) ^ 0x9E37_79B9;
    let x = (h1 % STARFIELD_W as u64) as f32;
    let y = (h2 % STARFIELD_H as u64) as f32;
    let angle = ((h1 >> 16) % 360) as f32 * std::f32::consts::PI / 180.0;
    let speed = 0.3 + ((h2 >> 8) % 71) as f32 / 100.0; // 0.3..1.0 cells/sec
    let brightness = ((h1 >> 24) % 200) as u8;
    let level = 70u8.saturating_add(brightness);
    let symbol = if brightness > 150 {
        '✦'
    } else if brightness > 80 {
        '·'
    } else {
        '.'
    };
    Particle {
        x,
        y,
        vx: angle.cos() * speed,
        vy: angle.sin() * speed,
        symbol,
        color: Color::Rgb {
            r: level,
            g: level,
            b: (level as u16 + 30).min(255) as u8,
        },
        lifetime: Duration::from_secs(STAR_LIFETIME_SECS),
        age: Duration::ZERO,
    }
}
```

Add fields to `Launcher` and initialize them in `new()`:

```rust
struct Launcher {
    location: Location,
    active: Option<Box<dyn App>>,
    selected: usize,
    nexus_phase: f32,
    starfield: ParticleSystem,
    star_seed: u64,
    returning: Option<Transition>,
    quit: bool,
}

impl Launcher {
    fn new() -> Self {
        Launcher {
            location: Location::Nexus,
            active: None,
            selected: 0,
            nexus_phase: 0.0,
            starfield: ParticleSystem::new(),
            star_seed: 0,
            returning: None,
            quit: false,
        }
    }
```

In `on_tick`'s `None` arm, add starfield updating/topping-up before the existing `returning` handling:

```rust
    fn on_tick(&mut self, elapsed: Duration) {
        match &mut self.active {
            Some(app) => app.on_tick(elapsed),
            None => {
                self.nexus_phase += elapsed.as_secs_f32();
                self.starfield.update(elapsed);
                while self.starfield.len() < TARGET_STAR_COUNT {
                    self.star_seed = self.star_seed.wrapping_add(1);
                    self.starfield.spawn(spawn_star(self.star_seed));
                }
                if let Some(t) = &mut self.returning {
                    t.tick(elapsed);
                    if t.is_complete() {
                        self.returning = None;
                    }
                }
            }
        }
    }
```

In `view`'s `None` arm, pass `&self.starfield` through to `nexus::render`:

```rust
            None => {
                let fade = self.returning.as_ref().map_or(1.0, |t| t.progress());
                nexus::render(self.selected, &self.starfield, self.nexus_phase, fade, area, buf);
            }
```

- [ ] **Step 4: Update `nexus.rs` to render the starfield and drop the old hash-grid**

Add `use ttui::particles::ParticleSystem;` to `nexus.rs`'s imports.

Change `render`'s signature and body — replace:

```rust
pub(crate) fn render(selected: usize, phase: f32, fade: f32, area: Rect, buf: &mut LayerStack) {
    if area.width < 12 || area.height < 10 {
        return;
    }
    let mut scene = Buffer::new(area.width, area.height);
    fill_void(&mut scene);
    starfield(&mut scene, phase);
    header(&mut scene);
    portals(&mut scene, selected, phase);
    footer(&mut scene);
```

with:

```rust
pub(crate) fn render(
    selected: usize,
    starfield: &ParticleSystem,
    phase: f32,
    fade: f32,
    area: Rect,
    buf: &mut LayerStack,
) {
    if area.width < 12 || area.height < 10 {
        return;
    }
    let mut scene = Buffer::new(area.width, area.height);
    fill_void(&mut scene);
    starfield.render(&mut scene);
    header(&mut scene);
    portals(&mut scene, selected, phase);
    footer(&mut scene);
```

Delete the entire old `fn starfield(scene: &mut Buffer, phase: f32) { ... }` function (the hash-grid twinkle implementation) — it's fully superseded.

- [ ] **Step 5: Fix the existing panic-safety test's call sites**

In `examples/launcher/main.rs`'s `nexus_render_does_not_panic_across_sizes` test, update both `nexus::render(...)` calls for the new signature:

```rust
    #[test]
    fn nexus_render_does_not_panic_across_sizes() {
        let starfield = ParticleSystem::new();
        for (w, h) in [(12, 10), (40, 15), (80, 24), (120, 40), (200, 60)] {
            let mut stack = LayerStack::new(w, h);
            let area = Rect {
                x: 0,
                y: 0,
                width: w,
                height: h,
            };
            for sel in 0..APP_COUNT {
                nexus::render(sel, &starfield, 1.23, 1.0, area, &mut stack);
                nexus::render(sel, &starfield, 5.0, 0.4, area, &mut stack);
            }
        }
    }
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --example launcher` (or the workspace-wide `cargo test` — this file's tests run as part of the `launcher` example's test target)
Expected: PASS, including the new `starfield_tops_up_to_target_count_after_ticking` test and the updated panic-safety test.

- [ ] **Step 7: Commit**

```bash
git add examples/launcher/main.rs examples/launcher/nexus.rs
git commit -m "feat(launcher): replace the static starfield with real ParticleSystem drift

The nexus's background was a static hash-grid that only twinkled in
place — the original launcher spec asked for particle-driven drift,
which this delivers. Stars spawn against a fixed virtual space (on_tick
has no access to the real terminal size) and turn over via a 30s
lifetime rather than wrapping at edges, since ParticleSystem exposes
no way to mutate an in-flight particle's position."
```

---

### Task 2: Real enlarge-on-focus

**Files:**
- Modify: `examples/launcher/nexus.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: nothing new downstream.

This task changes only `nexus.rs`'s `portals()` sizing — pixel output, the examples carve-out applies; the existing `nexus_render_does_not_panic_across_sizes` test (Task 1) is the regression check across the same size matrix, including the smallest (`12x10`) where the new clamps matter most.

- [ ] **Step 1: Implement per-portal sizing**

Replace:

```rust
fn portals(scene: &mut Buffer, selected: usize, phase: f32) {
    let w = scene.width;
    let h = scene.height;
    let slot_w = w / 3;
    if slot_w < 6 {
        return;
    }
    let box_w = slot_w.saturating_sub(2).max(5);
    let box_h = h.saturating_sub(8).clamp(5, 11);
    let top = 4 + h.saturating_sub(8).saturating_sub(box_h) / 2;

    for (i, &(name, tagline, accent)) in PORTALS.iter().enumerate() {
        let slot_x = i as u16 * slot_w + slot_w.saturating_sub(box_w) / 2;
        let slot = Rect {
            x: slot_x,
            y: top,
            width: box_w,
            height: box_h,
        };
        portal::draw(scene, slot, name, tagline, accent, i == selected, phase);
    }
}
```

with:

```rust
fn portals(scene: &mut Buffer, selected: usize, phase: f32) {
    let w = scene.width;
    let h = scene.height;
    let slot_w = w / 3;
    if slot_w < 6 {
        return;
    }
    let base_w = slot_w.saturating_sub(2).max(5);
    let base_h = h.saturating_sub(8).clamp(5, 11);
    let focus_w = (base_w + 2).min(slot_w.saturating_sub(1));
    let focus_h = (base_h + 1).min(h.saturating_sub(2));

    for (i, &(name, tagline, accent)) in PORTALS.iter().enumerate() {
        let focused = i == selected;
        let box_w = if focused { focus_w } else { base_w };
        let box_h = if focused { focus_h } else { base_h };
        let slot_x = i as u16 * slot_w + slot_w.saturating_sub(box_w) / 2;
        let top = 4 + h.saturating_sub(8).saturating_sub(box_h) / 2;
        let slot = Rect {
            x: slot_x,
            y: top,
            width: box_w,
            height: box_h,
        };
        portal::draw(scene, slot, name, tagline, accent, focused, phase);
    }
}
```

- [ ] **Step 2: Run the existing panic-safety test**

Run: `cargo test --example launcher nexus_render_does_not_panic_across_sizes`
Expected: PASS across all 5 sizes (including `12x10`, where `slot_w = 4` — note this is already below the function's own `slot_w < 6` early-return guard, so `portals()` draws nothing at that size, same as before this task; the smallest size where portals actually draw is where `focus_w`/`focus_h`'s clamps matter, and the test's other four sizes cover that).

- [ ] **Step 3: Visual check**

Run: `cargo run --example launcher` if you have a way to do so in this environment; if not (no interactive PTY), reason through the sizing at a couple of concrete `(w, h)` values (e.g. `80x24`) and confirm `focus_w > base_w` and `focus_h > base_h` without exceeding their clamps, and say so in your report.

- [ ] **Step 4: Commit**

```bash
git add examples/launcher/nexus.rs
git commit -m "feat(launcher): make the focused portal actually enlarge

The original launcher spec said the focused portal 'enlarges/pulses'
but only pulse (brightness) was ever implemented. Portal boxes now
size individually: +2 width / +1 height when focused, clamped to stay
within their slot and the available height."
```

---

### Task 3: Gradient portal borders

**Files:**
- Modify: `examples/launcher/portal.rs`

**Interfaces:**
- Consumes: `Theme.primary_end` (existing, from Arc A).
- Produces: nothing new downstream.

- [ ] **Step 1: Implement the conditional gradient**

Change:

```rust
    let theme = Theme {
        background: VOID,
        primary: border,
        secondary: accent,
        tertiary: accent,
        accent,
        primary_end: None,
        border: BorderSet {
```

to:

```rust
    let theme = Theme {
        background: VOID,
        primary: border,
        secondary: accent,
        tertiary: accent,
        accent,
        primary_end: if focused {
            Some(dim_color(accent, 0.3 + 0.7 * pulse))
        } else {
            None
        },
        border: BorderSet {
```

- [ ] **Step 2: Build**

Run: `cargo build --example launcher`
Expected: compiles cleanly (`dim_color` and `pulse` are already in scope in this function — `pulse` is computed at the top of `draw`, `dim_color` is already imported via `use crate::{dim_color, text_center, VOID};`).

- [ ] **Step 3: Visual check**

Run: `cargo run --example launcher` if possible; otherwise reason through it: confirm `focused == true` produces a `Some(...)` gradient target distinct from `border` (the flat color the ring's other end already uses) so the ring visibly ramps rather than staying flat, and say so in your report.

- [ ] **Step 4: Commit**

```bash
git add examples/launcher/portal.rs
git commit -m "feat(launcher): gradient border on the focused portal

primary_end was hardcoded to None because the field didn't exist when
this file was written. The focused portal's border now genuinely
ramps toward a pulse-modulated accent variant instead of rendering
flat, using the same pulse value already driving its brightness."
```

---

### Task 4: Dive-in flourish on launch

**Files:**
- Modify: `examples/launcher/main.rs`

**Interfaces:**
- Consumes: `ttui::transition::Transition`, `ttui::particles::{Particle, ParticleSystem}` (existing).
- Produces: `Launcher.diving` field — nothing downstream in this plan consumes it further.

This is the most structurally involved task: it moves the app-swap side effect out of `apply()` and into `on_tick`, and changes an EXISTING test's assertions as a direct, correct consequence (not a regression) — read the note in Step 3 carefully before touching it.

- [ ] **Step 1: Write the failing tests**

In `examples/launcher/main.rs`'s `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn launch_starts_a_dive_before_swapping_active_app() {
        let mut l = Launcher::new();
        l.apply(Action::Launch(1));
        assert!(l.diving.is_some());
        assert!(l.active.is_none());
        assert_eq!(
            l.location,
            Location::Nexus,
            "location doesn't change until the dive completes"
        );
    }

    #[test]
    fn dive_completes_into_the_active_app_after_enough_ticks() {
        let mut l = Launcher::new();
        l.apply(Action::Launch(1));
        l.on_tick(DIVE_DURATION + Duration::from_millis(10));
        assert!(l.active.is_some());
        assert_eq!(l.location, Location::Tardis);
        assert!(l.diving.is_none());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo build --example launcher`
Expected: FAIL to compile — `Launcher` has no `diving` field, `DIVE_DURATION` doesn't exist yet.

- [ ] **Step 3: Fix the existing `apply_launch_and_return_toggle_location` test**

**Why this is a required, correct change and not a workaround:** this task moves the location/active swap from `apply()` (synchronous) to `on_tick` (after the dive completes). The existing test's assertion `assert_eq!(l.location, Location::Tardis);` immediately after `apply(Action::Launch(1))` describes the OLD synchronous behavior and will be false once Step 4 lands — `location` now stays `Location::Nexus` until the dive finishes. Replace the test:

```rust
    #[test]
    fn apply_launch_starts_a_dive_apply_return_resets_to_nexus() {
        let mut l = Launcher::new();
        assert_eq!(l.location, Location::Nexus);
        l.apply(Action::Launch(1));
        assert_eq!(
            l.location,
            Location::Nexus,
            "location doesn't change until the dive completes"
        );
        assert!(l.diving.is_some());
        l.apply(Action::ReturnToNexus);
        assert_eq!(l.location, Location::Nexus);
        assert!(l.active.is_none());
        assert!(l.returning.is_some());
    }
```

- [ ] **Step 4: Implement the dive state machine**

Add constants near the other launcher constants:

```rust
const DIVE_DURATION: Duration = Duration::from_millis(400);
const DIVE_PARTICLE_COUNT: u32 = 16;
const NOMINAL_CENTER_X: f32 = 40.0;
const NOMINAL_CENTER_Y: f32 = 12.0;
```

Add this free function (near `spawn_star`):

```rust
/// Builds a short-lived particle burst approximating an "into the
/// portal" flourish for launching app `index`. Origin is a fixed
/// offset from a nominal center point, not the portal's real screen
/// position — `apply()` (called from `update()`) has no access to the
/// terminal's actual size, same constraint `spawn_star` works around.
fn spawn_burst(index: usize) -> ParticleSystem {
    let mut ps = ParticleSystem::new();
    let cx = NOMINAL_CENTER_X + (index as f32 - 1.0) * 20.0;
    let cy = NOMINAL_CENTER_Y;
    let accent = PORTALS[index].2;
    for i in 0..DIVE_PARTICLE_COUNT {
        let angle = i as f32 * (std::f32::consts::TAU / DIVE_PARTICLE_COUNT as f32);
        ps.spawn(Particle {
            x: cx,
            y: cy,
            vx: angle.cos() * 25.0,
            vy: angle.sin() * 12.0,
            symbol: '*',
            color: accent,
            lifetime: DIVE_DURATION,
            age: Duration::ZERO,
        });
    }
    ps
}
```

Add the field to `Launcher` and initialize it in `new()`:

```rust
struct Launcher {
    location: Location,
    active: Option<Box<dyn App>>,
    selected: usize,
    nexus_phase: f32,
    starfield: ParticleSystem,
    star_seed: u64,
    diving: Option<(usize, Transition, ParticleSystem)>,
    returning: Option<Transition>,
    quit: bool,
}
```

(add `diving: None,` to the `new()` constructor, alongside the other fields)

Change `apply()`'s `Action::Launch` arm from:

```rust
            Action::Launch(i) => {
                self.active = Some(make_app(i));
                self.location = location_of(i);
                self.returning = None;
            }
```

to:

```rust
            Action::Launch(i) => {
                self.diving = Some((i, Transition::start(DIVE_DURATION), spawn_burst(i)));
                self.returning = None;
            }
```

Change `on_tick`'s `None` arm (as it stands after Task 1) to also advance and resolve the dive — add this after the existing `returning` block:

```rust
                if let Some((_, transition, burst)) = &mut self.diving {
                    transition.tick(elapsed);
                    burst.update(elapsed);
                }
                let dive_complete = self.diving.as_ref().is_some_and(|(_, t, _)| t.is_complete());
                if dive_complete {
                    if let Some((index, _, _)) = self.diving.take() {
                        self.active = Some(make_app(index));
                        self.location = location_of(index);
                    }
                }
```

(This two-step "check completion, then `.take()`" shape avoids holding a mutable borrow of `self.diving` across the `self.active`/`self.location` writes — a single combined `if let Some((index, transition, burst)) = &mut self.diving { ...; self.active = ...; }` block risks a borrow-checker conflict since `self.active`/`self.location` are sibling fields being written while `self.diving` is still borrowed for `index`/`transition`/`burst`.)

Change `view`'s `None` arm to render the dive (dimmed nexus + burst overlay) when diving:

```rust
            None => {
                if let Some((_, transition, burst)) = &self.diving {
                    let fade = 1.0 - transition.progress();
                    nexus::render(self.selected, &self.starfield, self.nexus_phase, fade, area, buf);
                    let mut scene = Buffer::new(area.width, area.height);
                    burst.render(&mut scene);
                    for y in 0..scene.height {
                        for x in 0..scene.width {
                            let cell = scene.get(x, y);
                            if *cell != Cell::default() {
                                buf.set(area.x + x, area.y + y, cell.clone());
                            }
                        }
                    }
                } else {
                    let fade = self.returning.as_ref().map_or(1.0, |t| t.progress());
                    nexus::render(self.selected, &self.starfield, self.nexus_phase, fade, area, buf);
                }
            }
```

Finally, guard input against a launch-restart mid-dive — a real edge case: without this, pressing `Enter` again while already diving would call `route(Nexus, Enter, ...)` → `Action::Launch` → overwrite the in-progress dive with a fresh one, since `self.location` doesn't flip to the destination app until the dive completes (so `update()`'s top-level `self.location == Location::Nexus` check still routes through the nexus branch during a dive). In `update()`, change:

```rust
        if self.location == Location::Nexus {
            let action = route(Location::Nexus, key, self.selected, false);
            self.apply(action);
            return;
        }
```

to:

```rust
        if self.location == Location::Nexus {
            if self.diving.is_some() {
                return; // ignore input mid-dive, same spirit as an app ignoring input during its own boot
            }
            let action = route(Location::Nexus, key, self.selected, false);
            self.apply(action);
            return;
        }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --example launcher`
Expected: PASS, including both new tests, the corrected `apply_launch_starts_a_dive_apply_return_resets_to_nexus`, and every other pre-existing test in this file unaffected by this task.

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 7: Visual check**

Run: `cargo run --example launcher` if possible; otherwise reason through the sequence (press `Enter` → dive state active for `DIVE_DURATION` → app's own boot begins) against the code and say so in your report.

- [ ] **Step 8: Commit**

```bash
git add examples/launcher/main.rs
git commit -m "feat(launcher): add a dive-in particle flourish before launching an app

Launch now starts a brief (400ms) dive — a particle burst over the
dimming nexus — before the chosen app's own boot sequence begins,
instead of an instant cut. The app-swap side effect moves from apply()
(synchronous) to on_tick (fires when the dive transition completes);
apply_launch_and_return_toggle_location is updated accordingly since
its old assertion described the now-superseded synchronous behavior."
```

---

### Task 5: Final workspace verification

**Files:** none (verification only).

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: full suite green, including every test added/changed across Tasks 1-4.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds — the three sub-apps' own standalone `cargo run --example {omnitrix,tardis,smash_crabs}` entry points are untouched by this plan and must still build.

- [ ] **Step 4: Manual visual regression check**

Run `cargo run --example launcher`: confirm stars visibly drift; the focused portal is visibly larger with a genuine gradient border; pressing `Enter` shows a brief particle burst before the app's own boot begins; `F12`/app-`q`/nexus-`q` routing behaves exactly as before.

Run `cargo run --example omnitrix` / `tardis` / `smash_crabs`: confirm each still runs standalone, unaffected by this plan.

- [ ] **Step 5: Commit (if Step 4 required any fix) or proceed**

If Step 4 surfaces no issues, there is nothing to commit for this task.

---

## Final verification (whole plan)

- [ ] `cargo test` — full suite green.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo build --all-targets` — library, examples, benches all compile.
- [ ] Manual visual check on the launcher and all three standalone sub-apps confirms no regression.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this Arc's worktree branch to `main`, wait for all four required checks green, squash-merge, then remove the worktree via `ExitWorktree`.
