# TTUI Vision Alignment — Design (Rev B)

**Status:** Rev B (draft, pending your review before we move to planning).
**Date:** 2026-08-05
**Relationship to Rev A:** this is an addendum, not a replacement. Rev A
(`2026-08-04-ttui-core-framework-design.md`) is unchanged and fully
implemented on `main`; every guarantee it makes still holds. This spec
adds new, opt-in capability on top of it.

## Context / Motivation

`D:\Dev\Projects\TTUI-Ideas\vision\UI` holds three independent "vision"
documents — Omnitrix, Super Smash Crabs, and TARDIS — each pitching a
themed OS-shell skin (alien-tech, fighting-game, time-machine) over the
same three example apps (an agent/chat interface, a task-list app, a
system dashboard). None of the three reference a shared architecture
doc; each invents its own rendering assumptions independently. None
contain Rust code — architecture is described only in prose.

This spec reverse-engineers what Rev A's core is missing to make any of
these buildable, and commits to the smallest slice needed to validate
one of them (Omnitrix) end to end, while explicitly recording — but not
yet designing — what the other two would additionally need.

### What each vision needs beyond Rev A

Rev A's architecture is input-driven only (redraw happens strictly as a
consequence of a key/resize event), operates on a single `Buffer`, has
stateless widgets, and has no theme concept beyond raw
`crossterm::style::Color` values set per cell. All three visions need
things this doesn't provide:

- **Omnitrix** — an off-screen buffer diffed to stdout, which is
  already Rev A's model. New needs: a continuous "breathing pulse"
  animation (sine-wave brightness) independent of keypresses, a
  transition effect that briefly corrupts the buffer when switching
  apps, and a themed palette + thick custom borders.
- **Super Smash Crabs** — three named, z-ordered buffers (background/UI/
  effects) composited before flush; tweened/eased cursor movement;
  whole-buffer screen-shake; a themed palette and custom widget skins.
- **TARDIS** — a large virtual buffer (e.g. 500×500 cells) with a
  `Camera` (x, y, zoom, rotation) projecting a viewport onto stdout; a
  secondary decaying "Glitch Buffer" overlay; perspective/depth shading;
  a themed palette.

All three independently converge on the same two things: some form of
continuous, tick-driven animation, and a semantic color-role palette
(Background/Primary/Secondary/Tertiary/Accent) with swappable border
glyphs — neither of which Rev A has.

## Scope of this spec

**Committed and designed here:** an opt-in tick subscription mechanism,
and a minimal semantic `Theme`. Together these are sufficient to
validate the cheapest of the three visions, Omnitrix, end to end.

**Explicitly not designed here, recorded only as future direction:**
buffer layering/compositing (needed by Smash Crabs and TARDIS) and a
camera/viewport abstraction (needed by TARDIS). Both are gated on the
outcome of the Omnitrix validation prototype — see "Deferred" sections
below.

**Unchanged from Rev A, and not gaps:** focus/navigation routing (app
state already handles this — see "Not a gap" below), audio (stays an
app-level concern), and the single-dependency (`crossterm` only)
posture.

## Decision: opt-in tick subscription

Rev A states a real, deliberate commitment: "input-driven redraw, not
tick-based... render happens synchronously as a direct result of each
input event." Adding a generic animation tick is in tension with that
if read as a blanket rule — so this is recorded here as an explicit,
narrow exception, not a quiet reversal.

The existing event loop (`src/app.rs`) already polls with a timeout —
`term.next_event(Duration::from_millis(250))` — and currently does
nothing when that poll times out (the `if let Some(event) = ...` block
is simply skipped). That unused timeout branch becomes the tick
trigger:

- `App` grows a new trait method with a default implementation:
  `fn tick_rate(&self) -> Option<Duration> { None }`. Because it has a
  default body, existing `App` implementors (including
  `examples/demo.rs`) compile and behave unchanged without editing a
  single line — `None` means the loop's timeout stays a fixed
  housekeeping poll, exactly today's behavior, and no `on_tick` is ever
  called. A parallel default no-op `fn on_tick(&mut self, _elapsed: Duration) {}`
  is added for the same reason — implementing it is opt-in.
- When an app returns `Some(d)`, the loop's poll timeout becomes `d`.
  On each timeout, the loop calls a new `on_tick(&mut self, elapsed: Duration)`
  hook, then runs the same view → layout → paint → diff → flush pipeline
  already used after `update()`. A tick is a second trigger for the
  existing pipeline, not a second pipeline.
- Animation state (pulse phase, tween progress, decay counters) lives in
  app state and is mutated inside `on_tick`, exactly the way
  `list_selected` in `examples/demo.rs` is mutated inside `update()`
  today. It is then read into `view()`'s paint calls like any other app
  state field. **No change to the stateless-widget model is needed** —
  widgets still only ever see `(data, area)`, they have no idea whether
  the data they were handed came from a keypress or a tick.

**Open risk, explicitly not yet resolved by this spec:**
`Terminal::draw_diff` (`src/terminal.rs`) issues one `execute!` call
bundling `MoveTo`+`SetForegroundColor`+`SetBackgroundColor`+`Print` per
changed cell, with a single flush at the end. Running this at a 30-60Hz
tick rate with dozens of cells changing per tick (a pulsing border, a
tweened cursor) is untested. This spec does not assume it's fine — it
requires the Omnitrix validation prototype (see "Validation plan"
below) to measure it before the tick mechanism is trusted for anything
beyond a handful of animated cells per frame.

## Decision: minimal semantic Theme

All three vision docs independently converge on the same palette
shape: a Background, a Primary/active color, a Secondary/inactive
color, a Tertiary/alert color, and one Accent, each with its own hex
value — plus a per-theme choice of border glyphs and font. Today,
colors are raw `crossterm::style::Color` literals hardcoded into widget
code (e.g. `src/widgets/block.rs` hardcodes `'-'`/`'|'`/`'+'` for every
border, regardless of theme).

Design: a plain-data `Theme` struct with five `Color` fields
(`background`, `primary`, `secondary`, `tertiary`, `accent`) and an
optional `BorderSet` (one glyph per edge: top/bottom/left/right, plus
four corner glyphs) that `Block::render` uses instead of its current
hardcoded literals. `Theme` is threaded through the same
builder-pattern call sites `view()` already uses —
`Block::new().title(t)` becomes `Block::new().title(t).theme(&theme)` —
not through a global or through the `App` trait. `Theme` itself lives as
ordinary app state, the same way `Focus` does in `examples/demo.rs`
today, so switching themes at runtime (e.g. Omnitrix's app-switch
transition) is just an app-state write, no new plumbing.

Widget function signatures are otherwise unchanged. Note for future
reference: `Cell` (`src/buffer.rs`) currently has only
`{symbol, fg, bg}` — no style/attribute field (bold, underline, etc.).
That's out of scope here; nothing in the Omnitrix validation prototype
requires it.

## Not a gap: focus and navigation routing

Rev A's model — focus is a field in app state, `Tab` cycles it, and the
app's `update()` routes navigation keys to whichever widget is
currently focused, with **no framework-side focus manager** — already
covers all three visions' navigation metaphors without any core change:
Omnitrix's dial/scroll-then-launch, Smash Crabs' grid-token cursor, and
TARDIS's spatial rotate/pan are all, in shape, "an index or coordinate
in app state, advanced by an `update()` match arm" — exactly the
pattern `examples/demo.rs`'s `list_selected` already demonstrates. No
new abstraction is proposed; building a pluggable navigation-shell trait
now would be speculative generality for three metaphors that are
mutually exclusive by nature, with no fourth caller in sight.

## Deferred (documented, not designed): buffer layering

**Update (2026-08-05):** this has since been designed and shipped — see
`2026-08-05-buffer-layering-compositing-design.md`. The rest of this
section is preserved as the original historical record.

Smash Crabs wants three explicit, z-ordered buffers (background/UI/
effects) composited before diffing; TARDIS wants a single decaying
"Glitch Buffer" overlaid on the primary buffer. Neither is designed in
this spec. If and when this is built, the recorded direction is: Paint
writes into an ordered stack of same-dimension `Buffer`s instead of one,
and a **Composite** step is inserted at the seam Rev A already reserved
between Paint and Diff, flattening the stack into a single `Buffer`
immediately before Diff runs — Diff and the terminal writer are
untouched, since they only ever see the final flattened buffer.
Compositing uses discrete rules ("last non-default cell in stack order
wins"), not true alpha blending — `Cell` has no alpha channel, and
adding one is a larger change than layering itself; TARDIS's "50%
opacity" decay would need to be approximated by how many cells get
overwritten per tick, not real color math. **This entire section is
explicitly out of this spec's committed scope** — it is not built until
after the Omnitrix prototype validates the tick mechanism, and it has no
dependency on that prototype's outcome other than sequencing.

## Deferred (documented, not designed): camera/viewport

TARDIS wants a large virtual buffer with a `Camera` (x, y, zoom,
rotation) projecting a viewport onto the terminal each frame. The
recorded direction, if and when this is built: this lives at
**app-space**, not as core framework machinery. `Buffer::new(width, height)`
already supports an arbitrarily large virtual buffer with no core
change (e.g. `Buffer::new(500, 500)` works today). Pan/zoom would need
one small, additive helper — a "blit a W×H window starting at
`(camera.x, camera.y)` from the virtual buffer into the terminal-sized
buffer" operation — and perspective/depth shading (dimming/shrinking
toward the edges) would be a per-cell color transform applied during
that blit, at the app or a small library-helper level, not core.
Literal per-cell **rotation** does not fit `Layout`'s or `Buffer`'s
model at all and is flagged as likely out of scope entirely — "walking
around a hexagonal console" is recommended to be simulated (swap which
face's content is shown, plus pan/zoom) rather than implemented as true
per-cell rotation, unless a future spec commits to that as a
substantially bigger, separate piece of work. **This entire section is
explicitly out of this spec's committed scope.**

## Validation plan

Omnitrix is the target for the first working prototype, because its
rendering model already matches Rev A's (a single buffer, diffed and
flushed — no layering, no camera) and its two distinctive needs (the
breathing-pulse animation and the app-switch transition-corruption
effect) reduce entirely to the tick subscription plus ordinary
app-state-driven `view()` branching — nothing from the deferred sections
above is required.

Plan: re-skin or extend `examples/demo.rs` into an Omnitrix-flavored
example exercising exactly the two decisions in this spec — a
continuously pulsing border (tick-driven) and a themed palette
(Omnitrix's green/black scheme via the new `Theme` struct) — and nothing
else. Success criteria:

- The pulse animates smoothly at the app's chosen tick rate without
  visibly degrading input responsiveness (Tab/Up/Down still feel
  instant — Rev A's tactile-responsiveness commitment must hold for
  apps that use ticks too, not just apps that don't).
- `Terminal::draw_diff`'s per-cell `execute!` pattern is measured under
  this real animated load, resolving the open risk flagged above with
  actual numbers instead of assumption.
- The demo renders with the Omnitrix palette and border glyphs via
  `Theme`, with no widget-signature changes required.

Buffer layering and the camera abstraction are explicitly gated on this
prototype's outcome — neither is scoped for design work until the tick
mechanism and its performance characteristics are proven out on a real,
working example.

## Explicitly deferred / open questions for future revisions

- Buffer layering/compositing for Smash Crabs (see "Deferred" above) —
  not designed, direction recorded only — since designed and
  implemented, see `2026-08-05-buffer-layering-compositing-design.md`.
- Camera/viewport abstraction for TARDIS (see "Deferred" above) — not
  designed, direction recorded only; per-cell rotation flagged as
  likely permanently out of scope.
- `Cell` style/attribute support (bold, underline) — not needed by the
  Omnitrix validation prototype; revisit only if a later prototype
  needs it.
- Audio (`rodio` integration, wanted by Smash Crabs and TARDIS but
  absent from Omnitrix) — recorded as an app-level concern, not part of
  TTUI core, consistent with Rev A's single-dependency posture.
