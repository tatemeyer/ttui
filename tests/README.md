# Integration tests

Not used yet. Unit tests live inline via `#[cfg(test)] mod tests` in
each module — see
`docs/design/specs/2026-08-04-testing-verification-conventions-design.md`.
This directory is for integration tests that exercise the crate as an
external consumer would, via the public `ttui::` API across module
boundaries. Add a test file here the first time one is actually
needed, not before.
