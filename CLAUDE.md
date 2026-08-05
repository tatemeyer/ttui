# TTUI

Terminal UI framework project. Development follows the
[superpowers](https://github.com/obra/superpowers) methodology: no
implementation without an approved design doc (`docs/design/specs/`) and
plan (`docs/design/plans/`) first. See `docs/design/README.md` for how
specs and plans are structured (Arcs, Slices, Tasks). Diagrams (see
`.claude/rules/diagram-standards.md`) live under `docs/diagrams/`.

Project-specific conventions live in `.claude/rules/` — read them before
proposing any design or writing any code:

- `.claude/rules/development-conventions.md`
- `.claude/rules/diagram-standards.md`
- `.claude/rules/code-forge.md`
- `.claude/rules/git-github-standards.md`

Custom skills in `.claude/skills/` (`explore-codebase`, `debug-issue`,
`refactor-safely`, `review-changes`, `audit-graph-compliance`) wrap
[code-review-graph](https://github.com/tirth8205/code-review-graph)'s
MCP tools — prefer them over ad hoc grepping/reading when working in an
already-graphed codebase.

Core framework language is Rust; rendering model and v1 scope are decided
in `docs/design/specs/2026-08-04-ttui-core-framework-design.md` (Rev A) —
read it before proposing changes to the core architecture.
