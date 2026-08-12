# TTUI — Tate's Terminal User Interface

A terminal UI framework built from first principles: direct control over
text rendering, color, pane layout, and multiplexing.

**Status:** the core render pipeline (`App` state → view builder →
`Layout` → paint → diff → terminal writer), a constraint-based layout
engine, alpha-compositing buffer layering, and a growing set of
widgets and primitives — `Text`/`List`/`Table`/`Block` plus glitch
effects, particle systems, a fixed-forward perspective-projection
camera, a general key/chord input binder, and data-viz widgets among
them — are implemented and exercised by nine example apps: the
original `demo`/`omnitrix` core-framework smoke tests, four full
themed vision-doc apps (`tardis`, `smash_crabs`, `falcon`, and a
cross-app `launcher`), two research spikes, and a telemetry dashboard.
See `docs/design/README.md` for the full Arc history and
`examples/README.md` for what each example demonstrates.

## Try it

```
cargo run --example demo             # nested panes, Tab focus, Up/Down navigation
cargo run --example launcher         # cross-app portal nexus (omnitrix/tardis/smash_crabs)
cargo run --example falcon           # windshield + HUD + input-bound Easter egg
cargo run --example mission_control  # animated bar-chart/sparkline telemetry dashboard
```

See `examples/README.md` for the full list and what each one demonstrates.

## Design docs

- `docs/design/README.md` — the living index of Arcs (one line per
  subsystem or example-app bucket) and how specs, plans, and tasks
  relate.
- `docs/design/specs/2026-08-04-ttui-core-framework-design.md` (Rev A)
  and `docs/design/specs/2026-08-05-ttui-rev-b-vision-alignment-design.md`
  (Rev B) — the original core-framework and tick/theme design docs.
  Everything since is organized per-Arc under `docs/design/specs/<arc>/`.

## Workflow

This project is developed using [superpowers](https://github.com/obra/superpowers)
— every feature goes through brainstorm → design doc → plan → subagent-
driven implementation. See `.claude/rules/` for project-specific
conventions layered on top of that.

## Development knowledge graph

Codebase structure is tracked via [code-review-graph](https://github.com/tirth8205/code-review-graph)
(Tree-sitter-built, MCP-exposed). Run `code-review-graph build` after
cloning to populate it locally.
