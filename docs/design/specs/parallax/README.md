# Parallax — moved

The Parallax master design was written here and now lives in the
**Parallax** repository, per its own "Home: its own repository" note:

```
D:/Dev/Projects/Parallax/docs/design/specs/parallax/2026-08-14-parallax-platform-design.md
```

## Why TTUI still cares

Parallax is the platform binding TTUI, Model-Experiments, and Plumb into
one system. Its thesis is that **verification is the binding constraint
on agent autonomy** — an agent works unattended exactly as far as "done"
is machine-checkable — and its response is to build checkers for
progressively fuzzier things. Tiers 0–2 (compiles, tests, numeric
thresholds) are settled practice in both repos. Tier 3 — *looks right,
matches stated intent* — is the frontier, and [Plumb](../plumb/README.md)
is its first instance. Tier 4 (is a result novel or interesting) is named
unsolved and deliberately out of scope.

Three things in that document bear directly on this repo:

- **The three-axis autonomy model.** `.claude/rules/git-github-standards.md`'s
  Direct / Gated / Human tiers project onto `implement`, `merge`, and
  `readiness` rather than sitting on one ladder. The projection makes an
  asymmetry visible that neither repo could see about itself: TTUI has no
  human-only tier, and Model-Experiments has no direct-push tier.
- **Methodology stays untouched.** Parallax normalizes the vocabulary of
  *done* and *who may act*, never how work is planned. TTUI stays
  methodology-first; `methodology:` appears in the manifest as
  informational metadata only, and nothing in the platform branches on
  it. The two repos are a live controlled comparison of opposing theories
  of agentic development, and the platform's job is to make both
  observable through one lens, not to pick a winner.
- **The cockpit depends on `ttui` as a published crate**, making it
  TTUI's first genuine external consumer — exactly the API pressure a
  v1.0.0 needs, and which in-repo examples structurally cannot supply.

Nothing in this repo is governed by that document. It is filed here
because this is where it was designed and because TTUI is a consumer.
