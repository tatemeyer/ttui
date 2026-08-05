# Python Tooling Upgrade Procedure

Scoped to Python-based tooling in this repo (`code-review-graph`, and
any future scripts scaffolded from
`.claude/templates/repo/pyproject.toml.template`) — not TTUI's own
core language, which is Rust (see root `CLAUDE.md`).

## Standard procedure (uv-managed)

For any subtree with its own `pyproject.toml`:

```
cd <tooling-subtree>
uv lock --upgrade
uv sync
```

For globally-installed tool CLIs (e.g. `code-review-graph` itself,
installed as a standalone tool rather than a project dependency):

```
uv tool upgrade code-review-graph
```

## Known gotcha on this setup

`uv` was **not on `PATH`** during this project's initial bootstrap
(verified in both the background-job Bash shell and PowerShell). Check
before relying on it:

```
uv --version
```

If it's unavailable, fall back to plain `pip` against the same
interpreter Claude Code's tools resolve (`python`, not `python3` — this
laptop's `python3` alias is broken/absent):

```
python -m pip install --upgrade code-review-graph
```

This is how `code-review-graph` itself was installed during bootstrap.
Once `uv` is confirmed working in an interactive session, prefer it
going forward — it's the intended long-term path, this is a fallback,
not a replacement recommendation.
