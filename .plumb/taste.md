# TTUI — Taste Profile

The standard the `design` lens judges against. Read this as the
project's declared aesthetic, not as general UI advice — where the two
conflict, this file wins.

Structure is layered: this file states the grammar every TTUI app
obeys. A themed app that deliberately departs from it declares the
departure in its own scenario entry (see "Per-app overrides" below).
Absent an override, this file governs.

## Aesthetic intent

**A machine you are operating, lit up in a dark room.**

TTUI apps are props — instruments, consoles, cockpits, gadgets. The
user is not reading a document or filling in a form; they are sitting
inside something that is running. Every app is a different machine
(a Ben 10 gadget, a TARDIS console, a smuggler's cockpit, a ground
control station), but they are all *machines*, and they are all
*powered on*.

The governing line: **loud in colour and motion, disciplined in
structure.**

## Non-negotiables

Violating any of these makes a frame wrong regardless of which app it
belongs to. All four are legitimate grounds for a blocking-severity
finding when the breach is unambiguous.

1. **Legibility survives the effects.** Glow, shake, particle bursts
   and transitions may never render actual state or data unreadable.
   Light it up as much as you like — the reading still has to be
   readable at the moment it matters. An effect that obscures the
   value it decorates has failed, however good it looks.

2. **Cell-grid discipline.** Everything aligns to the terminal grid.
   Borders close, corners meet, panels do not drift by a cell, edges
   are not ragged, nothing overlaps that did not mean to. This is the
   most common real defect in terminal UI work and the easiest thing
   in this document to check.

3. **Colour carries state.** The palette communicates status — armed,
   charging, damaged, nominal, selected — not merely decoration. A
   frame where colour has stopped meaning anything has lost
   information the operator needs, even if it is pretty.

4. **It reads as a machine.** Panels, bezels, depth, instruments. If a
   frame could pass for a settings dialog, a form, or a web page, it is
   wrong no matter how clean it is.

## Deliberate violations of generic UI norms

Generic UI heuristics are wrong about TTUI in exactly two places. Do
not raise findings on either.

- **Constant motion.** Glow pulses, particle bursts, screen shake,
  starfields, radar sweeps. Nothing is ever fully still. Standard
  advice to minimise motion, or to respect reduced-motion preferences,
  does not apply here — motion is the product, not an enhancement to
  it.

- **Saturation and glow.** Bright, saturated, high-contrast palettes
  with bloom and chromatic bleed. Standard advice preferring muted,
  low-fatigue palettes does not apply — the target is a lit console in
  a dark room, not a document in daylight.

## Explicitly still open to critique

Stated positively, because a taste profile that only grants permissions
teaches the lens nothing.

TTUI does **not** claim density or ornament as deliberate violations.
That means:

- **Packed layouts are fair game.** Density is often correct — an
  instrument panel *should* be busy — but it is not automatically
  correct. A frame that is cluttered rather than dense, where the eye
  cannot find the thing it needs, is a legitimate finding.
- **Ornament is fair game.** Decorative borders, dingbats and gradient
  rings serve the machine fantasy, but ornament that competes with the
  state it surrounds, or that exists only because it was easy to draw,
  is a legitimate finding.

The two lists above are the whole point of this file: motion and colour
are settled, structure is not.

## Intentional distortion

Some TTUI apps deliberately corrupt the display — `src/glitch.rs`, and
Falcon's percussive-maintenance mechanic, garble glyphs and displace
regions on purpose.

This is the one place the taste profile and the `breakage` lens can
collide, because deliberate corruption is indistinguishable from a
rendering bug by inspection alone. It is resolved at the scenario
level, not here: a scenario that expects corruption declares it, and
the breakage lens is told which distortion is intended. A scenario that
does *not* declare it gets the default treatment, and garbled output is
a defect.

Intentional distortion is still bound by non-negotiable 1: a glitch
that permanently destroys a reading, rather than momentarily disturbing
it, is a defect regardless of the declaration.

## Per-app overrides

A themed app that departs from this grammar declares the departure in
its scenario entry rather than by amending this file. Overrides are
additive and scoped to that scenario.

```yaml
scenarios:
  - name: falcon-glitch-burst
    intent: >
      Percussive maintenance triggers a corruption burst across the
      windshield; the instrument panel readings remain legible
      throughout.
    expects:
      - visual-corruption      # breakage lens: distortion is intended here
    taste_override: >
      Falcon is the scruffiest machine in the set — worn, patched,
      visibly repaired. Wear and asymmetry are correct here in a way
      they would not be for mission_control.
```

No per-app overrides are authored yet. They are added when the design
lens actually misfires on a specific app, not in advance of evidence —
eight speculative profiles would be eight guesses about mistakes that
have not happened.
