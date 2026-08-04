# TTUI — Tate's Terminal User Interface

A terminal UI framework built from first principles: direct control over
text rendering, color, pane layout, and multiplexing.

**Status:** bootstrapping. The harness (`.claude/`, doc workflow, tooling
templates) is being stood up first; the framework's own design has not
started yet.

## Workflow

This project is developed using [superpowers](https://github.com/obra/superpowers)
— every feature goes through brainstorm → design doc → plan → subagent-
driven implementation. See `.claude/rules/` for project-specific
conventions layered on top of that.

## Development knowledge graph

Codebase structure is tracked via [code-review-graph](https://github.com/tirth8205/code-review-graph)
(Tree-sitter-built, MCP-exposed). Run `code-review-graph build` after
cloning to populate it locally.
