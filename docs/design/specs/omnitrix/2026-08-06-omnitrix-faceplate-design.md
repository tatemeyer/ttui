# Omnitrix Faceplate Dial-Navigation Hub (Issue #42) — Design

**Status:** draft, pending your review.
**Date:** 2026-08-06
**Relationship to prior specs:** implements the Structural-wave ticket of
Arc 1 (Omnitrix) from `2026-08-06-example-apps-roadmap-design.md` (issue
#42, tracking #52). Builds on the shipped glow-border work
(`2026-08-06-omnitrix-glow-border-design.md`, issue #41, PR #85) and Rev
B's tick-driven `Theme`/`Block` foundation
(`2026-08-05-ttui-rev-b-vision-alignment-design.md`). `examples/omnitrix.rs`
is the only file this spec touches — no `src/` changes.

## Problem

`examples/omnitrix.rs` today is a single static screen (the pulsing
"Recharge Pulse" border) with no navigation. The roadmap's vision-doc
source describes a "Faceplate": a scrollable list of DNA-sample apps
(Brainstorm/Fasttrack/Upgrade), navigated with Tab, launched by "slamming
the dial down." This spec adds that navigation hub as a second, selectable
screen state within the existing example — not a new widget, not a new
crate module.

## Scope

Two related things, both confined to `examples/omnitrix.rs`:

1. A `Faceplate` screen: a 3-item selectable list (reusing `List`,
   `src/widgets/list.rs`, unmodified) with Tab/Shift+Tab cycling and an
   Enter "slam to launch" trigger.
2. A `Launched` screen: a placeholder shown after launching a sample,
   since the three sub-apps (Brainstorm #48, Fasttrack #49, Upgrade #50)
   and the real `AppMode` state enum (#43) are separate, not-yet-built
   tickets. This spec deliberately builds a minimal, local, throwaway
   stand-in for that switching rather than leaving "launch" a no-op —
   scoped to this example only, not a public API, and not a substitute
   for #43's own design.

**Explicitly out of scope:**
- Real scrolling — the list is fixed at 3 items, which always fits;
  `List` is used exactly as it exists today (YAGNI).
- Real sub-app content — `Launched` shows a name and placeholder text
  only.
- The "corruption" transition effect on switch (issue #44) — switching
  is an instant cut, no animation, in this spec.
- Any change to `src/widgets/list.rs`, `Theme`, or `Block`.

## Design

### State

```rust
#[derive(Clone, Copy, PartialEq)]
enum DnaSample {
    Brainstorm,
    Fasttrack,
    Upgrade,
}

impl DnaSample {
    const ALL: [DnaSample; 3] = [DnaSample::Brainstorm, DnaSample::Fasttrack, DnaSample::Upgrade];

    fn name(&self) -> &'static str {
        match self {
            DnaSample::Brainstorm => "Brainstorm",
            DnaSample::Fasttrack => "Fasttrack",
            DnaSample::Upgrade => "Upgrade",
        }
    }
}

enum Screen {
    Faceplate,
    Launched(DnaSample),
}
```

`Omnitrix` gains two fields: `selected: usize` (index into
`DnaSample::ALL`, starts at `0`) and `screen: Screen` (starts at
`Screen::Faceplate`). Existing fields (`pulse_phase`, `quit`,
`last_tick_started`, `perf_log`) are unchanged — the pulse keeps
animating regardless of screen.

### Interaction (`update()`)

`q` quits unconditionally, from either screen — unchanged.

On `Screen::Faceplate`:
- `Tab`: `selected = (selected + 1) % 3`
- `KeyCode::BackTab` (crossterm's shift+tab): `selected = (selected + 2) % 3`
- `Enter`: `screen = Screen::Launched(DnaSample::ALL[selected])`

On `Screen::Launched(_)`:
- `Esc`: `screen = Screen::Faceplate` (selection is preserved — the user
  lands back where they were, not reset to index 0)

Tab/Shift+Tab/Enter are ignored on `Screen::Launched`; Esc is ignored on
`Screen::Faceplate` (no binding, falls through the existing `let
Event::Key(k) = event else { return }` / kind-check guard unchanged).

### Rendering (`view()`)

The existing pulsing `Block` (title "Omnitrix", themed border from #41)
wraps both screens unchanged — same call as today,
`Block::new().title("Omnitrix").theme(&theme).render(area, buf)`.

Inside the returned inner `Rect`:
- `Screen::Faceplate`: build `[String; 3]` from `DnaSample::ALL[i].name()`
  and render via `List::new(&names, self.selected).render(inner, buf)`.
- `Screen::Launched(sample)`: render `sample.name()` as a line via
  `Text::new(...)`, followed by a placeholder line — e.g.
  `Text::new("(not yet built)")` — on the row below. Simplest layout:
  two direct `Text::new(...).render(...)` calls against two
  vertically-split sub-`Rect`s of `inner` (`inner.height` is at least 2
  in any reasonably-sized terminal; no `Layout`/`Constraint` machinery
  needed for two fixed rows).

### Testing

Pure example code — `examples/omnitrix.rs` only, no `src/` changes.
Verified by running `cargo run --example omnitrix`, per the TDD
exceptions in `.claude/rules/development-conventions.md` ("Examples/demos...
correctness is checked by running the example, not asserting on it"),
consistent with every other change to this file to date (Rev B, #41).

## Verification

- `cargo build --example omnitrix`, `cargo fmt`, `cargo clippy --all-targets`
  clean.
- `cargo run --example omnitrix`: confirm Tab/Shift+Tab cycles the
  highlighted sample and wraps at both ends; Enter on each of the 3
  samples shows that sample's name on a placeholder screen; Esc returns
  to the Faceplate with the same selection still highlighted; the border
  keeps pulsing/bolding on both screens; `q` quits from both screens with
  a clean exit.
