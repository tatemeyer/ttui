# Cross-App Launcher (Portal Nexus) — Design

**Status:** draft, pending your review before we move to planning.
**Date:** 2026-08-08
**Relationship to prior specs:** net-new work beyond
`docs/design/specs/core/2026-08-06-example-apps-roadmap-design.md`,
whose backlog is now feature-complete (Omnitrix, TARDIS, Smash Crabs
each ship a hub + three sub-apps + boot + transitions). This spec adds
a *fourth* example that composes the existing three into one shell; it
introduces no new core (`src/`) capability and changes no app's
behavior beyond a mechanical visibility refactor.

## Context / Motivation

The three example apps are each a self-contained `App` (`ttui::app::
App`) run via `ttui::app::run`. They demonstrate the framework building
*one* themed OS-shell each — but nothing demonstrates the framework
composing *whole apps* into a larger shell, switching between them with
their own boot sequences and transitions intact. A "portal nexus"
launcher does exactly that: it is the framework eating its own dog food
one level up, and a strong single-command demo (`cargo run --example
launcher`) of everything built so far.

The hard part is not visual — it is **reusing three existing example
crates' `App` types from a fourth example** without duplicating their
code or moving their logic into the `ttui` library (the apps are
app-space, not framework, per Rev B's app-space boundary). Cargo
examples are independent binary crates with no first-class cross-example
imports, so the reuse mechanism is the central design decision here.

## Scope of this spec

**Committed:**
1. A `launcher` example: a themed **portal nexus** (its own fourth
   mini-theme) that lists the three apps as portals, launches a chosen
   app, and returns to the nexus.
2. The **code-reuse mechanism** letting the launcher construct and run
   each existing app's `App` type, while each `cargo run --example
   <app>` keeps working standalone.
3. **Event routing:** a reserved global key returns to the nexus from
   anywhere; each app's own `q` returns to the nexus; only the nexus's
   `q` exits the process.
4. **Transitions:** launching an app plays *that app's own* boot +
   signature transition (fresh instance per entry); returning plays a
   short launcher-owned fade.
5. Incidental cleanup: remove Omnitrix's leftover `perf_log`/
   `omnitrix_perf.log` side effect (superseded by the criterion
   benchmark from the render-diff perf arc); a composed launcher must
   not spew per-app log files.

**Explicitly out of scope:** any `src/` (framework) change; adding new
widgets or effects to the core; a new *fourth themed app* (this shell
reuses the three that exist); persistence of sub-app state across
return trips (each entry is a fresh instance, by design — see below).

## Design

### 1. Code-reuse mechanism (the crux)

Each app is refactored **mechanically** (no logic change) so its `App`
type is reachable both from its own thin entry point and from the
launcher:

- `examples/<app>/<app>.rs` — a new *app module* holding what
  `main.rs` holds today: the struct, constants, `impl` blocks, the
  `impl App`, and the screen sub-module declarations. The screen mods
  are rewritten from `mod boot;` to `#[path = "boot.rs"] mod boot;`
  (one per screen) so they keep resolving to the **existing** sibling
  files unmoved. The app struct and its constructor become
  `pub(crate)` (e.g. `pub(crate) struct Omnitrix` / `pub(crate) fn
  new()`); the screen files' `use super::*` continues to resolve to
  this module unchanged.
- `examples/<app>/main.rs` — reduced to a thin entry:
  ```rust
  #[path = "<app>.rs"]
  mod app;
  fn main() -> std::io::Result<()> {
      ttui::app::run(&mut app::<Struct>::new())
  }
  ```

The launcher includes each app module by path:
```rust
#[path = "../omnitrix/omnitrix.rs"]     mod omnitrix;
#[path = "../tardis/tardis.rs"]         mod tardis;
#[path = "../smash_crabs/smash_crabs.rs"] mod smash_crabs;
```
and constructs each via `omnitrix::Omnitrix::new()`, etc. A nested
`#[path]` on a screen mod resolves relative to the directory of the
file physically containing the `mod` statement (each app module's own
directory), so the launcher's inclusion does not disturb screen-file
resolution.

**Why this mechanism** over the alternatives:
- *Move apps into `ttui` (src/)* — rejected: pollutes the framework
  crate with app-space code and violates Rev B's app-space boundary.
- *Include each `main.rs` directly via `#[path]`* — rejected: each
  `main.rs` has a `fn main` that becomes dead code (fails the
  `-D warnings` gate) and a private struct; the thin-entry split avoids
  both cleanly.

**Primary risk:** nested `#[path]` resolution + `use super::*` under
double inclusion is subtle. **De-risk:** the plan's first task is a
throwaway compile spike that composes *one* app into a trivial launcher
and confirms it builds, before refactoring all three.

### 2. Launcher `App` architecture

A single `struct Launcher` implements `App` and is what `run` drives.
It owns:

- `location: Location` — `Nexus | Omnitrix | Tardis | SmashCrabs`.
- The active sub-app instance (owned, `Option<Box<dyn App>>` or an
  enum of the concrete types), **created fresh on entry and dropped on
  return** so the app's own boot sequence replays each time.
- Nexus state (selected portal index, idle-animation phase).
- A launcher-owned `Transition` for the return fade.
- `quit: bool` (set only from the nexus).

Delegation while in an app: `tick_rate`, `on_tick`, and `view` forward
to the active instance. In the nexus, `tick_rate` is the nexus's own
animation interval, `on_tick` advances the portal animation / return
fade, and `view` renders the nexus.

### 3. Event routing (return + quit)

`Launcher::update` routes, in order:

1. **Reserved global return key = `F12`** — intercepted *before*
   delegation, from anywhere inside an app: start the return fade and
   drop the active instance. Guaranteed to work even while an app
   ignores input during its own boot/transition, because the launcher
   sees the event first.
2. **In an app:** delegate the event to the active instance's
   `update`, then check its `should_quit()`. If the app now wants to
   quit (its own `q` handling set it), the launcher reinterprets that
   as *return to nexus*, not process exit — so each app's existing `q`
   means "back," with no edit to the app's own update logic.
3. **In the nexus:** arrows/`Tab` move the portal selection, `Enter`
   launches the selected app (fresh instance), and `q` sets
   `launcher.quit` → `should_quit()` true → `run` exits the process.

`should_quit()` returns `self.quit` only; an app's internal quit never
propagates past the launcher.

### 4. Nexus UX + theme (themed multiverse/portal chooser)

The nexus is a fourth mini-`Theme` (void/deep-space background, a
shifting portal accent). It composites via `LayerStack`:

- **background layer** — a slow starfield/void (reusing `particles`
  for drift).
- **portals layer** — three portals, one per app, each tinted with
  that app's signature accent (Omnitrix green, TARDIS blue, Smash
  Crabs red) and labeled; the focused portal enlarges/pulses via
  `easing` hover.
- **UI layer** — title + a hint row (`←/→` or `Tab` select · `Enter`
  launch · `F12` back · `q` quit).

A brief nexus power-up on process start is optional and kept minimal;
the visual weight belongs to the three apps.

### 5. Transitions

- **Enter:** launching creates a fresh app instance, whose constructor
  already sets `booting: Some(..)` (all three do), so the app's own
  boot + signature transition plays with no new effect code. An
  optional short "dive into portal" launcher flourish before handoff is
  allowed but not required.
- **Return:** a launcher-owned quick fade (a `Transition` driving
  `camera::dim` or a fade-to-void overlay) back to the nexus.

### 6. Known tradeoffs

- **Fresh instance per entry** means sub-app state resets on return and
  each entry re-initializes that app's `RodioAudioSink`. This is the
  intended behavior (boot replays) and keeps the launcher stateless
  about sub-app internals; rapid enter/exit churns audio streams
  briefly — acceptable for a demo.
- The launcher does not attempt to pass terminal-resize or focus events
  differently than `run` already does; it is a pure `App` and inherits
  `run`'s existing loop semantics.

## Testing

Per `.claude/rules/development-conventions.md`, examples are a TDD
exception (correctness by running) — with one carve-out that **is**
test-first: the event-routing decision is extracted as a pure function

```rust
fn route(location, key, app_wants_quit) -> Action  // Stay | Launch(i) | ReturnToNexus | QuitProcess
```

so it can be unit-tested off a real TTY (the `F12`/app-`q`/nexus-`q`
matrix). The nexus rendering, portal visuals, and transitions are the
demo exception. The three per-app refactors are behavior-preserving and
verified by running each standalone example plus `cargo build
--examples`. Real-TTY manual verification (headless CI cannot run it)
covers the composed launcher.

## Critical files

- Add: `examples/launcher/main.rs` (+ `nexus.rs`, `portal.rs` split to
  keep each file under the 500-line ceiling).
- Modify: `examples/{omnitrix,tardis,smash_crabs}/main.rs` — split into
  a thin entry + a new `<app>.rs` app module (`pub(crate)` struct +
  constructor, `#[path]` screen mods).
- Modify: `examples/omnitrix/main.rs` / app module — remove `perf_log`.
- Modify: `examples/README.md` — add the `launcher` entry.
- Modify: `docs/design/README.md` — add the `launcher/` Arc.
- No `src/` changes.

## Verification

- `cargo run --example launcher` — nexus shows three portals; `Enter`
  launches the selected app through its own boot; `F12` returns from
  anywhere; an app's `q` returns to the nexus; the nexus's `q` exits.
- `cargo run --example omnitrix` / `tardis` / `smash_crabs` — each
  still runs standalone after the refactor.
- `cargo build --examples`, `cargo clippy --all-targets -- -D
  warnings`, `cargo fmt --check`, `cargo test` — all green (the `route`
  unit tests included).
- No `omnitrix_perf.log` is created by any example.
- Every new example file is under the 500-line soft ceiling.
