# Authoring a Plumb capture scenario

Read this before writing a new scenario. It exists because every
scenario authored so far broke at least one of these rules on the
first attempt, and the breakage was silent — no error, no non-zero
exit code, nothing to distinguish a working capture from a wrong one
except reading the image.

## The one rule that matters most

**Exit code 0 is not evidence of success.** Two of the first three
"successful" captures in this repo exited 0 while showing the wrong
screen entirely (boot animation, not the intended UI). The only real
check is: read the contact sheet and describe, in your own words, what
is actually on it — every attempt, not just the first. If you can't
describe what's in every tile, you haven't verified the capture.

## Before writing a script

1. **Read the boot constant.** Grep `BOOT_MS` / `BOOT_TOTAL_MS` (sum
   the parts if it's composed of several constants). First `wait_ms`
   must clear it with margin. `omnitrix-dial-rotate`'s original script
   sent its entire 1300ms of keys inside a 2500ms boot window and
   every frame it ever captured was boot animation.
2. **Read the input match arm for the screen you're targeting** —
   not the on-screen hint text, which can lag or simplify the real
   bindings. The same scenario sent `Right`/`Right` at a screen whose
   arm binds `Tab`/`BackTab`; the keys were silent no-ops for two
   review passes before anyone noticed.
3. **Check where each key actually lands.** A key that transitions to
   another screen can land somewhere the rasterizer can't draw. The
   same script's trailing `Enter` opened a sub-screen using `\xe2\x97\x8f`/`\xe2\x97\x8b`
   glyphs (Geometric Shapes block), which hard-errors the whole
   capture with no image written at all.
4. **Grep the target screen(s) for non-ASCII.** Covered:
   ASCII/Latin-1, Box Drawing, Block Elements, Braille Patterns, and a
   handful of misc codepoints. Not covered at all: Geometric Shapes
   (U+25A0-25FF), Arrows (U+2190-21FF), Dingbats, anything above
   U+FFFF. If the target screen needs one of those, the scenario
   **must** use `adapter: pty` with `on_unmapped_glyph: substitute` —
   see below, this is not optional and there is no fallback under
   `adapter: command`.
5. **Prefer no input, then single keys, then a single click.** Avoid
   chords. See "Chords" below — this isn't a style preference, chords
   are not reliably scriptable yet.

## `on_unmapped_glyph: substitute` only works under `adapter: pty`

It does not exist in `tools/visual-snapshot` at all — it's a
plumb-only field, and only `capture_pty` reads it. Declaring it on an
`adapter: command` scenario (which shells out to `visual-snapshot`)
parses fine and does nothing; the capture still hard-errors on the
first unmapped glyph it hits, indistinguishable from having declared
nothing. If a screen needs substitute mode, the whole scenario has to
be `adapter: pty`.

## Chords are not reliably scriptable yet (ttui#138)

The settle wait between two `key` steps is patient — it waits for the
screen to change and then hold steady, up to 2000ms. On a
continuously-animating screen it can take anywhere from milliseconds
to the full 2000ms, unpredictably. If the app's chord window (e.g. a
1500ms `CHORD_TIMEOUT`) is shorter than that gap, the chord silently
resets and never fires. A run that misses looks identical on exit code
to one that hit — the only way to tell is reading the contact sheet.
Stick to single keys until this is fixed.

## Writing `intent`

**`intent` must describe what the capture SHOWS, not what the app
IS.** Write it after reading the contact sheet, never before. Five
over-claimed intents have already had to be corrected across three
scenarios: a gauge that actually renders on a different screen than
the one captured, a label that isn't positioned where memory said, a
colour claimed to be state-driven that turns out to be a tick-driven
pulse. An intent lens judges the rendered image against this sentence
and nothing else — over-claiming makes it fire on a non-defect,
under-claiming makes it useless as a gate. If the capture shows
something narrower or less interesting than you expected going in,
narrow the intent to match. Don't round up.

## `expects` and `touches`

- `expects: [visual-corruption]` only for distortion the app produces
  *deliberately* — describe the arc (peak, then clear), not a
  legibility claim the peak frame itself contradicts.
- Narrow `touches` to files the scenario's captured screen(s) actually
  render. A glob like `src/widgets/**` on a scenario that draws two
  widgets is a false-selection generator: it hands a lens agent an
  image with no bearing on whatever else changed under that glob and
  invites a confident verdict on evidence that isn't there.

## Artifacts

A multi-frame capture (any script with 1+ steps) produces three
files: the per-frame `.gif`, a tiled contact-sheet `.png` — **this is
what lens agents actually read; they cannot decode GIFs** — and the
run manifest. A zero-step script produces one frame and only a `.png`.
Always read the contact sheet, not the gif.

## Known-broken example, do not copy

`falcon-glitch-burst`'s script sometimes captures only the ambient
single-panel idle flicker instead of the intended three-panel burst —
that's the chord problem above, not a regression in `src/glitch.rs`.
If a review run shows only single-panel flicker there, it's a capture
miss, not a finding.
