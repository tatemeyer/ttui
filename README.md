# TTUI — Tate's Terminal User Interface

A terminal UI framework built from first principles: direct control over
text rendering, color, pane layout, and multiplexing.

**Status:** v1 core is implemented — a five-stage, input-driven render
pipeline (`App` state → view builder → `Layout` → paint → diff →
terminal writer), a constraint-based layout engine, and a `Text`/
`List`/`Table`/`Block` widget set, proven out by `examples/demo.rs`.
An opt-in animation tick and a minimal `Theme` were added on top
("Rev B"), validated by `examples/omnitrix.rs`.

## Try it

```
cargo run --example demo      # nested panes, Tab focus, Up/Down navigation
cargo run --example omnitrix  # tick-driven pulsing themed border
```

## Design docs

- `docs/design/specs/2026-08-04-ttui-core-framework-design.md` (Rev A)
  — the core render pipeline, layout engine, widget set, and
  input-driven event loop.
- `docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md`
  (Rev B) — an opt-in tick subscription and a minimal semantic `Theme`,
  reconciling ideas from a sibling `TTUI-Ideas` vision repo against the
  Rev A core. Buffer layering and a camera/viewport abstraction remain
  deferred, pending further validation.
- `docs/design/README.md` explains how specs and their implementation
  plans (Arc → Slice → Task) relate.

## Workflow

This project is developed using [superpowers](https://github.com/obra/superpowers)
— every feature goes through brainstorm → design doc → plan → subagent-
driven implementation. See `.claude/rules/` for project-specific
conventions layered on top of that.

## Development knowledge graph

Codebase structure is tracked via [code-review-graph](https://github.com/tirth8205/code-review-graph)
(Tree-sitter-built, MCP-exposed). Run `code-review-graph build` after
cloning to populate it locally.
