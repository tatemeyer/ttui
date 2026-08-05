# Diagram Standards

**Status:** stub — notation/tooling per category still undefined, needs a
`/superpowers:brainstorm` pass. Storage location is decided (below).

Ported from the categories used on a prior project; the actual notation
and tooling per category still need deciding.

## Storage location

Diagrams live under `docs/diagrams/` (decided 2026-08-04), a separate
tree from `docs/design/specs/` and `docs/design/plans/` — not inline in
design docs. Layout within `docs/diagrams/` (e.g. subfolders per
category) is still open, along with everything in "Open questions"
below.

## Categories

- **DAGs** — dependency/build/task graphs.
- **Contracts** — interface/API boundary diagrams between components.
- **Domain / class models** — structural models of core domain types.
- **Packages with dataflows** — module boundaries plus how data moves
  between them.
- **Sequences (on-demand)** — built only when needed to explain a
  specific interaction, not maintained as living documentation.
- **Runtimes (on-demand)** — same as sequences: generated to explain a
  specific runtime behavior, not kept evergreen.
- **Built-vs-planned** — a way of visually distinguishing what exists
  today from what's designed but not yet implemented.

## Open questions to resolve via brainstorming

- Notation/tooling per category (e.g. Mermaid vs Graphviz/dot vs
  hand-drawn) — `obra/superpowers` itself uses Graphviz `dot` for
  process-flow diagrams in its own skill files, which may be a
  reasonable default to inherit rather than introduce a second tool.
- What "built-vs-planned" actually looks like visually (color coding?
  separate layers? a status annotation convention?).
