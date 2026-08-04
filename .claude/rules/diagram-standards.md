# Diagram Standards

**Status:** stub — not yet defined. Needs a `/superpowers:brainstorm` pass.

Ported from the categories used on a prior project; the actual notation,
tooling, and storage location for each category still need deciding.

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
- Where diagrams live relative to `docs/superpowers/specs/` — inline in
  the design doc, or a separate `docs/diagrams/` tree.
- What "built-vs-planned" actually looks like visually (color coding?
  separate layers? a status annotation convention?).
