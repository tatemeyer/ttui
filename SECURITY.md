# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| 1.1.x | Yes |
| 1.0.x | No — please upgrade; 1.1 is backward compatible |
| < 1.0 | No |

## Reporting a vulnerability

**Please do not open a public issue for a security vulnerability.**

Report it privately through GitHub's
[private vulnerability reporting](https://github.com/tatemeyer/ttui/security/advisories/new),
which notifies the maintainer without disclosing the issue publicly.

Please include:

- What the issue is and roughly how severe you think it is
- Steps to reproduce, or a proof of concept
- The `ttui` version and platform you saw it on

## What to expect

This is a solo-maintained project, so response times are best-effort
rather than contractual. Expect an initial acknowledgement within about a
week. If a report is confirmed, the fix and an advisory will be published
together, and you will be credited unless you'd rather not be.

## Scope

`ttui` is a terminal UI library. It has no network stack, no
deserialization of untrusted input, and no privileged operations — so the
realistic surface is narrow. Things genuinely worth reporting:

- Escape sequences in app-supplied strings that can escape the intended
  render region and drive the host terminal (injection through
  `Text`/`List`/`Table` content)
- A panic or crash path that leaves the terminal in raw mode, since that
  can render a user's shell unusable
- Memory-safety issues, including any misuse of unsafe code in a
  dependency that `ttui` surfaces

The bundled example apps and `tools/visual-snapshot` are development
material, not part of the published library's supported surface.
