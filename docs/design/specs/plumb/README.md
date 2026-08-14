# Plumb — moved

The Plumb design spec and its implementation plan were written here and
now live in the **Parallax** repository, per the spec's own "Home: its
own repository" note:

```
D:/Dev/Projects/Parallax/docs/design/specs/plumb/2026-08-14-plumb-design.md
D:/Dev/Projects/Parallax/docs/design/plans/plumb/2026-08-14-plumb-plan.md
```

## What it is, and what it changes here

`.claude/rules/development-conventions.md`'s "Visual review" section
mandates capturing a rendering-affecting change and judging whether it
looks right. The capture half is solved — `tools/visual-snapshot`. The
judgment half is not, for three compounding reasons: the tool doesn't
travel (it takes `--example <name>` and builds via `cargo build
--example`), an agent reviewing output it just produced is a soft grader,
and critique with no declared target regresses to stock advice — more
whitespace, calmer color — which for TTUI is actively wrong.

Plumb answers all three: portable capture adapters, an adversarial
multi-lens reviewer that **never sees the code it is judging**, and a
per-project taste profile the critique is measured against.

## What stays in this repo

- **`.plumb/taste.md`** — TTUI's declared aesthetic, and the standard the
  `design` lens judges against. Authored, and living here because it is
  this project's taste, not the tool's. Its shape: *a machine you are
  operating, lit up in a dark room*; four non-negotiables (legibility
  survives the effects, cell-grid discipline, colour carries state, it
  reads as a machine); two exemptions from generic UI advice (constant
  motion, saturation and glow); and density and ornament left explicitly
  open to critique, because a taste profile that only grants permissions
  teaches the lens nothing.
- **`tools/visual-snapshot` is not replaced.** TTUI adopts Plumb through
  the `command` adapter and keeps its existing tool verbatim and
  unmodified. That is a stated non-goal of the spec, not an accident of
  sequencing.
- On adoption (the plan's Arc 6): `.plumb/config.yaml` and
  `.plumb/scripts/*.json` land here, plus an additive note in
  `.claude/rules/development-conventions.md`.

Two tasks in the plan execute **in this repo** rather than in Parallax —
the seed scenario at the end of Arc 2, and all of Arc 6 — on a TTUI
worktree branch through the normal Gated PR flow.

## The one thing worth knowing about the design

Lens agents receive the image, the run manifest, the taste profile, and —
for the intent lens only — the scenario's declared `intent`. They do not
receive the diff, the source, the adapter's arguments, or even the fact
that anything changed, and their agent definitions declare `tools: Read`
and nothing else so a lens physically cannot grep its way to the source.
An agent that can read the code reasons *"the code draws three panes, so
there are three panes"* instead of looking. Blinded, it has nothing to do
but see.
