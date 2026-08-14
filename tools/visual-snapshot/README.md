# visual-snapshot

Headless visual review for TTUI example apps and `[[bin]]` targets
(e.g. `showcase`). Spawns a compiled example or bin binary under a real
OS pseudo-console (`portable-pty`, ConPTY on Windows), drives it with a
scripted sequence of key presses and real-time waits, captures its
terminal output via `vt100`, and rasterizes the result to a PNG (single
frame) or animated GIF (multiple frames) — an image an agent can `Read`
directly, without a real interactive TTY.

See `docs/design/specs/core/2026-08-09-visual-snapshot-tooling-design.md`
for the full design rationale, and
`.claude/rules/development-conventions.md`'s "Visual review" section
for when running this tool is mandatory.

## Usage

```
cargo run -p visual-snapshot -- --example <name> --size <cols>x<rows> --script <path.json> --out <path>
cargo run -p visual-snapshot -- --bin <name> --size <cols>x<rows> --script <path.json> --out <path>
```

- `--example <name>` — an example binary name under `examples/`
  (`launcher`, `omnitrix`, `tardis`, `smash_crabs`, ...). Built
  automatically via `cargo build --example <name>` before spawning.
- `--bin <name>` — a `[[bin]]` target name (e.g. `showcase`), as
  opposed to `--example`, an `[[example]]` target. Built automatically
  via `cargo build --bin <name>` before spawning. Exactly one of
  `--example`/`--bin` is required.
- `--size <cols>x<rows>` — pseudo-console size, e.g. `120x40`. Defaults
  to `80x24`.
- `--script <path.json>` — path to a script file (see below).
- `--out <path>` — output file path. Its extension must match what the
  script actually produces (see "Output format" below).

## Script format

A script is a flat JSON array of steps, each one of three shapes:

- `{"wait_ms": N}` — sleep `N` real milliseconds before the next
  capture. This is what actually exercises an app's `tick_rate()`-driven
  animation, since the tool never calls `on_tick()` directly — only real
  wall-clock time passing does.
- `{"key": "Right"}` — send a named key to the spawned example. Known
  names: `Up`, `Down`, `Left`, `Right`, `Enter`, `Esc`, `Tab`, any single
  ASCII character (`"a"`, `"Q"`, `"5"`), and `Ctrl+<letter>` combos
  (`"Ctrl+C"`). See `src/keys.rs` for the exact table.
- `{"x": N, "y": N}` — send a left-button click at cell `(x, y)`
  (0-indexed) to the spawned example.

Example — navigate right, wait for a transition, then confirm:

```json
[
  { "wait_ms": 300 },
  { "key": "Right" },
  { "wait_ms": 150 },
  { "key": "Enter" }
]
```

One frame is captured before the first step runs (the initial screen
state) plus one frame per step, so an `N`-step script always produces
`N + 1` frames.

## Output format

Frame count determines format, and `--out`'s extension must agree with
it:

- **0 steps → 1 frame → `--out` must end in `.png`.**
- **1+ steps → 2+ frames → `--out` must end in `.gif`.**

A mismatch is a hard error naming the actual frame count and the
extension it requires, rather than silently writing the wrong bytes
under a misleading name.

## Judging a screenshot

An optional, on-demand, local vision-model judgment step — advisory
only, never wired into CI, never a replacement for the mandatory human/
reviewer-subagent visual review required before merge
(`.claude/rules/development-conventions.md`'s "Visual review" section).
Useful for fast iteration while developing an example, or as a second
opinion alongside a full review.

**Prerequisites:** [Ollama](https://ollama.com) installed and running
locally, with a vision-capable model pulled:

```
ollama pull moondream
```

**Judge an already-captured screenshot:**

```
cargo run -p visual-snapshot -- judge <path.png> [--context "description"] [--model <name>]
```

**Judge immediately after capturing** (judges the final frame — for a
multi-step script, that's the end state after all steps run):

```
cargo run -p visual-snapshot -- --example <name> --script <path.json> --out <path> --review [--context "description"]
```

- `--context "description"` — tells the model what the screenshot is
  supposed to show. Without it, the model can only catch gross
  corruption (garbled glyphs, overlapping text) — it has no notion of
  what "correct" means for a specific example otherwise.
- `--model <name>` — Ollama model to use. Defaults to `moondream`
  (small, CPU-friendly). Override with a more capable model (e.g.
  `llava`) if you have a GPU.
- A judge failure (Ollama unreachable, model not pulled, malformed
  response) is printed to stderr and never changes `--review`'s
  capture success or exit code — judging and capturing are independent
  outcomes.

The fixed prompt sent to the model: "You are reviewing a screenshot of
a terminal UI rendered by an automated test tool. [This is supposed to
show: `{context}`.] Look for: garbled or missing glyphs, broken layout
(overlapping text, content cut off unexpectedly), or anything that
looks visually wrong. Respond with a brief verdict (LOOKS OK / POSSIBLE
ISSUE) followed by 1-3 sentences of reasoning."

## Known glyph-coverage limitation

The rasterizer maps each terminal cell's character to an 8x8 bitmap via
the `font8x8` crate, plus an algorithmic renderer for Braille Patterns
glyphs (`U+2800`-`U+28FF`, used by `TimeRotor`) that `font8x8` doesn't
cover at all. Some glyphs TTUI's examples draw are still unmapped by
either path and produce a **hard error** naming the codepoint — no
image is written — rather than a silently blank or wrong glyph. As of
this writing this affects `EnergyCore`'s charged-state dingbat star
(`✦`, also used separately by `launcher`'s starfield), several
decorative glyphs in the `launcher` example, `tardis`'s psychic relay
log (em dash), and `smash_crabs`'s explosion emoji (`💥`). See
`docs/design/specs/core/2026-08-09-visual-snapshot-tooling-design.md`'s
"Resolved during planning" section for the current, complete gap list,
and `.claude/rules/development-conventions.md`'s "Visual review"
section for what to do when a mandated review hits this.
