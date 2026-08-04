# TTUI

Terminal UI framework project. Development follows the
[superpowers](https://github.com/obra/superpowers) methodology: no
implementation without an approved design doc (`docs/superpowers/specs/`)
and plan (`docs/superpowers/plans/`) first.

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

Core framework language/architecture (Rust vs Go, rendering model, etc.)
is not yet decided — that is the subject of the first real
`/superpowers:brainstorm` session, not something to assume from this
file.
