# Cross-App Launcher (Portal Nexus) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for the pure-logic task and superpowers:executing-plans to drive the plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/design/specs/launcher/2026-08-08-cross-app-launcher-design.md`:
a `launcher` example that composes the three existing apps into a themed
portal nexus, with per-app-boot enter, fade return, and `F12`/`q`
routing — reusing each app in place with no `src/` change.

**Architecture:** A research spike proves the reuse mechanism first;
then a mechanical per-app refactor exposes each app module; then the
launcher `App` (with a unit-tested pure router) and the nexus visuals
land; then transitions and cleanup. All work is in `examples/` — no
framework change.

**Tech Stack:** Rust, `crossterm`, existing `ttui` crate
(`app`/`buffer`/`layout`/`theme`/`transition`/`easing`/`particles`/
`camera`). No new dependencies.

## Global Constraints

- No `src/` (framework) changes anywhere in this Arc.
- The per-app refactor is **behavior-preserving** — each `cargo run
  --example <app>` must produce identical behavior after the split.
- Keep every new/modified example file under the 500-line soft ceiling
  (split `nexus.rs`/`portal.rs` out of `launcher/main.rs`).
- Examples are a TDD exception except the pure `route` function, which
  is test-first.

---

## Slice 0: De-risk the reuse mechanism (`research`)

### Task 1: Compile spike — compose one app into a trivial launcher

**Files:**
- Temporary throwaway (deleted at task end).

`research`-tagged spike (deleted before ship, per the TDD spike
exception).

- [ ] **Step 1: Prove nested `#[path]` + `use super::*` composes.**
  Temporarily split `examples/omnitrix` per Slice 1's shape, add a
  throwaway `examples/spike/main.rs` that `#[path]`-includes
  `../omnitrix/omnitrix.rs` and constructs `omnitrix::Omnitrix::new()`,
  and confirm `cargo build --example spike` **and** `cargo run
  --example omnitrix` both compile. Record the exact `#[path]` forms
  that work.
- [ ] **Step 2: Delete the spike**, keeping only the confirmed
  mechanism notes to apply in Slice 1. If the mechanism does *not*
  compile, stop and revise the spec before proceeding.

---

## Slice 1: Per-app module refactor (`coding`, behavior-preserving)

### Task 2: Split each app into a thin entry + `pub(crate)` app module

**Files:**
- Modify: `examples/omnitrix/main.rs`; Add: `examples/omnitrix/omnitrix.rs`
- Modify: `examples/tardis/main.rs`; Add: `examples/tardis/tardis.rs`
- Modify: `examples/smash_crabs/main.rs`; Add: `examples/smash_crabs/smash_crabs.rs`

`coding`-tagged; TDD exception applies (mechanical move + example code;
verified by running). Do all three identically.

- [ ] **Step 1: Move struct + impls + screen mods** from each
  `main.rs` into a sibling `<app>.rs`, rewriting `mod boot;` →
  `#[path = "boot.rs"] mod boot;` (one per screen) so sibling files
  resolve unmoved. Make the app struct and its constructor
  `pub(crate)`.
- [ ] **Step 2: Reduce each `main.rs`** to `#[path = "<app>.rs"] mod
  app;` + a `fn main` that runs `app::<Struct>::new()`.
- [ ] **Step 3: Verify parity** — `cargo run --example omnitrix`,
  `tardis`, `smash_crabs` each build and behave as before; `cargo build
  --examples` clean.

---

## Slice 2: Launcher core + routing (`coding`, router is TDD)

### Task 3: Pure event router (test-first)

**Files:**
- Add: `examples/launcher/main.rs` (router fn + `Location`/`Action`).

`coding`-tagged, **TDD required** for the pure function.

- [ ] **Step 1 (RED): Write unit tests** for `route(location, key,
  app_wants_quit) -> Action` covering: `F12` in any app → `ReturnTo
  Nexus`; app `q` (app_wants_quit) → `ReturnToNexus`; nexus `Enter` on
  index i → `Launch(i)`; nexus `q` → `QuitProcess`; nexus arrows/`Tab`
  → `Stay` (selection handled separately); unrelated keys in an app →
  `Stay`.
- [ ] **Step 2 (GREEN): Implement `route`** and the `Location`/`Action`
  enums to satisfy the tests.

### Task 4: `Launcher` App wiring + delegation

**Files:**
- Modify: `examples/launcher/main.rs`.

`coding`-tagged; demo exception for the wiring (covered by running).

- [ ] **Step 1: Define `struct Launcher`** owning `location`, the
  active sub-app instance (created fresh on `Launch`, dropped on
  `ReturnToNexus`), nexus selection state, the return `Transition`, and
  `quit`.
- [ ] **Step 2: Implement `App` for `Launcher`** — `update` calls
  `route` then applies the `Action` (with nexus selection handled for
  arrow/`Tab`); `view`/`on_tick`/`tick_rate` delegate to the active
  instance in an app and to the nexus otherwise; `should_quit` returns
  `self.quit`. Include `#[path]` inclusion of the three app modules.

---

## Slice 3: Nexus visuals + theme (`coding`, demo exception)

### Task 5: Portal nexus rendering

**Files:**
- Add: `examples/launcher/nexus.rs`, `examples/launcher/portal.rs`.

`coding`-tagged; demo exception (correctness by running).

- [ ] **Step 1: Nexus `Theme`** (void background + portal accent) and a
  `LayerStack` composite: starfield background (`particles`), portals
  layer, UI/hint layer.
- [ ] **Step 2: Portal widget/helper** — three portals tinted per app
  accent, focused portal pulses via `easing`; title + hint row with the
  controls.

---

## Slice 4: Transitions + cleanup (`coding`)

### Task 6: Enter-via-boot, fade return, and incidental cleanup

**Files:**
- Modify: `examples/launcher/main.rs`; `examples/omnitrix/omnitrix.rs`;
  `examples/README.md`; `docs/design/README.md`.

`coding` + `admin`; demo exception for the transition visuals.

- [ ] **Step 1: Enter** — `Launch(i)` constructs a fresh instance so
  its own boot plays; **Return** — drive a launcher-owned fade
  `Transition` (via `camera::dim`/overlay) before showing the nexus.
- [ ] **Step 2: Remove Omnitrix's `perf_log`/`omnitrix_perf.log`**
  side effect (field, `OpenOptions` open, and the write site).
- [ ] **Step 3: Index updates** — add the `launcher` entry to
  `examples/README.md` and the `launcher/` Arc line to
  `docs/design/README.md`.

---

## Verification (whole plan)

- [ ] `cargo run --example launcher` — nexus → `Enter` launches through
  the app's own boot → `F12` returns from anywhere → app `q` returns →
  nexus `q` exits.
- [ ] `cargo run --example {omnitrix,tardis,smash_crabs}` each still
  runs standalone.
- [ ] `cargo test` green (the `route` unit tests included), `cargo
  clippy --all-targets -- -D warnings` and `cargo fmt --check` clean,
  `cargo build --examples` clean.
- [ ] No `omnitrix_perf.log` is produced by any example.
- [ ] Every new example file is under the 500-line soft ceiling.
- [ ] Manual (real-TTY): the composed launcher renders and navigates
  correctly (headless CI cannot run it; note the result per the
  real-TTY policy).
