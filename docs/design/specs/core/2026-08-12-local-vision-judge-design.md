# Local Vision Judge — Design

**Status:** draft, pending review before we move to planning.
**Date:** 2026-08-12
**Relationship to prior specs:** builds on `tools/visual-snapshot`'s
existing headless capture pipeline (`src/pty.rs`, `src/encode.rs`) —
this Arc adds a third capability to that tool (capture → click-scripting
→ now judging) rather than inventing new infrastructure. Prompted by a
conversation about how far the project is from automated visual review;
that conversation concluded the capture side was already solved and the
open gap was *judgment*, not capture.

## Problem

Every PR touching rendering-affecting code requires a human or reviewer
subagent to run `tools/visual-snapshot`, then `Read` the resulting
PNG/GIF and judge whether it looks right
(`.claude/rules/development-conventions.md`'s "Visual review" section).
That's a real, working process — but it has no fast path: getting a
second opinion, or a quick sanity check while actively iterating on an
example, currently means invoking a full reasoning subagent (or waiting
for one) just to answer "does this look broken?"

This spec adds a **local, on-demand** vision judgment step — not a CI
gate, not a replacement for the mandatory review, just a fast/cheap tool
available for two specific use cases: iterating quickly on an example
without spinning up a subagent for every tweak, and getting an
independent second opinion alongside the existing review.

## Scope

**Tag: `coding`, TDD mandatory** for everything except the literal
network call to a real Ollama instance, which is real-external-service
exempt (see Testing below, and the precedent already established for
"Real-TTY tests" in `development-conventions.md`).

Three slices, in dependency order:

1. **`judge` module** (`tools/visual-snapshot/src/judge.rs`) — prompt
   construction, request/response building, and the actual HTTP call to
   Ollama.
2. **CLI integration** (`tools/visual-snapshot/src/main.rs`,
   `src/encode.rs`) — depends on 1. A `judge` subcommand plus
   `--review`/`--context`/`--model` flags on the existing capture path.
3. **Documentation** (`tools/visual-snapshot/README.md`,
   `.claude/rules/development-conventions.md`) — depends on 1-2.

## Design

### Why Ollama, why `moondream` as the default

Ollama is the standard way to run a local model cross-platform
(Windows included) behind a stable, simple HTTP API
(`localhost:11434`) — no bespoke model-loading code needed in this
project at all; `tools/visual-snapshot` just becomes an HTTP client.
Hardware for this varies by machine and is often CPU-only, so the
documented default model is `moondream` (~1.8B, built for speed over
raw capability) to keep the "fast iteration" use case actually fast on
a CPU-only floor. `--model` overrides it for anyone with a GPU who
wants more capable judgment. The tool does not install Ollama or pull
any model itself — it assumes both are already present and fails with
a clear, actionable message if not.

### Slice 1: `judge` module

`tools/visual-snapshot/src/judge.rs` — new file. Pure logic (prompt
text, request-body shape, response parsing) is separated from the one
function that actually performs network I/O, so everything except the
live network call is unit-testable without a running Ollama instance:

```rust
//! Talks to a local Ollama instance (`localhost:11434`) to get a vision
//! model's judgment on a rendered terminal-UI screenshot. Advisory
//! only — never wired into CI, never a merge gate. See
//! `.claude/rules/development-conventions.md`'s "Visual review" section
//! for how this relates to the mandatory human/reviewer-subagent review.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;

const OLLAMA_GENERATE_URL: &str = "http://localhost:11434/api/generate";

/// Default local model — small enough for CPU-only inference to stay
/// fast; override with `--model` when a GPU or a more capable model is
/// available.
pub const DEFAULT_MODEL: &str = "moondream";

/// Failure judging a screenshot: reading the file, reaching Ollama, or
/// parsing its response.
#[derive(Debug)]
pub enum JudgeError {
    /// Couldn't read the image file from disk.
    Io(std::io::Error),
    /// Couldn't reach Ollama, or Ollama returned an error response
    /// (e.g. model not pulled) — the string is Ollama's own message
    /// where available, surfaced as-is rather than reinterpreted.
    Request(String),
    /// Ollama's response body wasn't the JSON shape expected.
    Parse(String),
}

impl From<std::io::Error> for JudgeError {
    fn from(e: std::io::Error) -> Self {
        JudgeError::Io(e)
    }
}

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JudgeError::Request(msg) => write!(
                f,
                "could not reach Ollama at {OLLAMA_GENERATE_URL} — is it running? ({msg})"
            ),
            _ => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for JudgeError {}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

/// Builds the fixed judging prompt, injecting `context` (a caller-
/// supplied description of what the screenshot is supposed to show)
/// when given. Without context the model can only catch gross
/// corruption (garbled glyphs, overlap) — it has no notion of what
/// "correct" means for a specific example otherwise.
pub fn build_prompt(context: Option<&str>) -> String {
    let mut p = String::from(
        "You are reviewing a screenshot of a terminal UI rendered by an automated test tool. ",
    );
    if let Some(c) = context {
        p.push_str(&format!("This is supposed to show: {c}. "));
    }
    p.push_str(
        "Look for: garbled or missing glyphs, broken layout (overlapping text, content cut \
         off unexpectedly), or anything that looks visually wrong. Respond with a brief \
         verdict (LOOKS OK / POSSIBLE ISSUE) followed by 1-3 sentences of reasoning.",
    );
    p
}

/// Builds the JSON body Ollama's `/api/generate` endpoint expects:
/// `stream: false` so the response comes back as one JSON object
/// rather than newline-delimited streaming chunks, sidestepping a
/// streaming parser entirely.
fn build_request_body(image_bytes: &[u8], context: Option<&str>, model: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "prompt": build_prompt(context),
        "images": [STANDARD.encode(image_bytes)],
        "stream": false,
    })
}

/// Parses Ollama's response body into the model's judgment text.
fn parse_response_body(body: &str) -> Result<String, JudgeError> {
    let parsed: OllamaResponse =
        serde_json::from_str(body).map_err(|e| JudgeError::Parse(e.to_string()))?;
    Ok(parsed.response)
}

/// Sends `body` to Ollama and returns the raw response text. Thin I/O
/// glue with no logic of its own to unit-test — see Testing in the
/// design spec for why this one function is verified manually instead.
fn send_request(body: &serde_json::Value) -> Result<String, JudgeError> {
    ureq::post(OLLAMA_GENERATE_URL)
        .send_json(body.clone())
        .map_err(|e| JudgeError::Request(e.to_string()))?
        .into_string()
        .map_err(|e| JudgeError::Request(e.to_string()))
}

/// Judges an in-memory PNG's bytes, returning the model's judgment
/// text. Used by `--review` (judging a just-captured frame without a
/// disk round-trip) and by `judge_file` below.
pub fn judge_png_bytes(
    image_bytes: &[u8],
    context: Option<&str>,
    model: &str,
) -> Result<String, JudgeError> {
    let body = build_request_body(image_bytes, context, model);
    let response_body = send_request(&body)?;
    parse_response_body(&response_body)
}

/// Judges an already-captured PNG file on disk. Used by the `judge`
/// subcommand.
pub fn judge_file(
    path: &std::path::Path,
    context: Option<&str>,
    model: &str,
) -> Result<String, JudgeError> {
    let bytes = std::fs::read(path)?;
    judge_png_bytes(&bytes, context, model)
}
```

New dependencies in `tools/visual-snapshot/Cargo.toml` (this crate
only — same isolation principle already established for
`portable-pty` never touching the root `ttui` crate):

```toml
ureq = "2"
base64 = "0.22"
```

### Slice 2: CLI integration

`tools/visual-snapshot/src/encode.rs` gains a small helper so
`--review` can judge the last captured frame without writing it to
disk first:

```rust
/// Encodes a single frame as in-memory PNG bytes, for callers (like
/// `--review`) that need the encoded bytes without writing to disk.
pub fn png_bytes(img: &RgbaImage) -> Result<Vec<u8>, EncodeError> {
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    img.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(buf)
}
```

`tools/visual-snapshot/src/main.rs` gains a `judge` subcommand and
three new flags on the existing (default) capture path. The existing
flat `cargo run -p visual-snapshot -- --example X --script Y --out Z`
invocation — used throughout this project's docs, PR verification
notes, and every prior Arc's visual-review step — keeps working
unchanged: `command` is `Option<Command>`, so when no subcommand word
is given, the existing capture logic runs exactly as it does today.

```rust
use clap::{Parser as ClapParser, Subcommand};
use visual_snapshot::{encode, judge, pty, script};

#[derive(ClapParser)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    // Capture path (existing behavior; unused when `command` is Some).
    #[arg(long)]
    example: Option<String>,
    #[arg(long, default_value = "80x24")]
    size: String,
    #[arg(long)]
    script: Option<std::path::PathBuf>,
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// After capturing, ask a local Ollama vision model to judge the
    /// final frame. Advisory only — capture still succeeds and writes
    /// `--out` even if this fails or Ollama is unreachable.
    #[arg(long)]
    review: bool,
    /// Description of what the capture is supposed to show, passed to
    /// the judge model as context. Used by both `--review` and `judge`.
    #[arg(long)]
    context: Option<String>,
    /// Ollama model name to judge with.
    #[arg(long, default_value = "moondream")]
    model: String,
}

#[derive(Subcommand)]
enum Command {
    /// Ask a local Ollama vision model to judge an already-captured screenshot.
    Judge {
        image: std::path::PathBuf,
        #[arg(long)]
        context: Option<String>,
        #[arg(long, default_value = "moondream")]
        model: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if let Some(Command::Judge { image, context, model }) = args.command {
        let verdict = judge::judge_file(&image, context.as_deref(), &model)?;
        println!("{verdict}");
        return Ok(());
    }

    let example = args.example.ok_or("--example is required")?;
    let (cols, rows) = parse_size(&args.size)?;
    let script_path = args.script.ok_or("--script is required")?;
    let out = args.out.ok_or("--out is required")?;

    let binary = pty::build_example(&example)?;
    let steps = script::parse_script(&script_path)?;
    let frames = pty::run_script(&binary, rows, cols, &steps)?;

    validate_output_extension(&out, frames.len())?;
    if frames.len() == 1 {
        encode::write_png(&frames[0].0, &out)?;
    } else {
        encode::write_gif(&frames, &out)?;
    }
    println!("wrote {} frame(s) to {}", frames.len(), out.display());

    if args.review {
        let last_frame = &frames.last().expect("run_script always returns 1+ frames").0;
        let png_bytes = encode::png_bytes(last_frame)?;
        match judge::judge_png_bytes(&png_bytes, args.context.as_deref(), &args.model) {
            Ok(verdict) => println!("--- judge review ---\n{verdict}"),
            Err(e) => eprintln!("--- judge review failed (capture above is still valid) ---\n{e}"),
        }
    }

    Ok(())
}
```

Note the `Err` arm: a judge failure is printed to stderr and does
**not** change `main`'s exit code — capture success and judge success
are independent outcomes, per the "never a gate" principle. (`args`
here also needs `example`/`script`/`out` changed from `String`/
`PathBuf` to `Option<...>` at the struct level, shown above; the
existing `parse_size`, `expected_extension`, and
`validate_output_extension` functions are untouched. Missing
`--example`/`--script`/`--out` now fails via a manual `.ok_or(...)`
message rather than clap's auto-generated "required argument missing"
text — still a clear, immediate failure, just worded by this code
instead of by clap, since these fields can no longer be non-`Optional`
once a subcommand coexists with them.)

The two `default_value = "moondream"` literals above must be kept in
sync with `judge::DEFAULT_MODEL` — clap's derive macro wants a literal
here, not an arbitrary path expression, so it can't simply reference
the constant directly. A unit test asserting
`judge::DEFAULT_MODEL == "moondream"` (Slice 1) pins this so the two
can't silently drift.

### Slice 3: Documentation

`tools/visual-snapshot/README.md` gains a new section covering:
prerequisites (`ollama pull moondream`, Ollama running), the `judge`
subcommand and `--review`/`--context`/`--model` flags, and the fixed
prompt template so users know what's actually being asked.

`.claude/rules/development-conventions.md`'s "Visual review" section
gains a short additive paragraph: `judge`/`--review` is available as an
optional, non-blocking aid for fast iteration and a second opinion —
explicitly not a replacement for the mandatory human/reviewer-subagent
visual review step still required before merge. Also documents the
real-external-service exemption from Testing below.

## Non-goals

- **Any CI involvement.** This never runs in CI, is never a required
  check, and never gates a build or a merge. No `.github/workflows/`
  changes.
- **Golden-image diffing.** Judgment is standalone per screenshot, no
  stored reference images, no golden-image maintenance burden. (A
  future Arc could add this; explicitly out of scope here.)
- **Bundling, installing, or auto-pulling a model.** The tool assumes
  Ollama and a model are already present; it only calls the API.
- **Replacing the mandatory human/reviewer-subagent visual review.**
  This is a fast/cheap supplement for iteration and a second opinion,
  not a substitute for the existing required step.
- **Structured/machine-parseable judge output.** Plain text to stdout;
  nothing downstream parses it programmatically.

## Testing

Per `.claude/rules/development-conventions.md`, `coding`-tagged work is
TDD-mandatory, with one narrow, named exception this spec adds:

- **TDD, fully covered:** `build_prompt` (with and without context),
  `build_request_body` (correct `model`/`stream: false`/base64-encoded
  `images` shape), `parse_response_body` (valid response → correct
  text; malformed JSON → `JudgeError::Parse`), `encode::png_bytes`
  (round-trip: encode then re-decode with the `image` crate, compare
  pixels), and the CLI's new arg parsing (subcommand recognized,
  `--review`/`--context`/`--model` flags parsed, existing flat
  invocation still parses with `command: None`).
- **Real-external-service exempt (new, narrow exception):**
  `send_request`'s actual HTTP round-trip to a real, running Ollama
  instance. Unit-testable request/response *logic* is fully covered
  above; only the literal "does a live local Ollama actually answer"
  path is exempt, for the same reason `development-conventions.md`
  already exempts real-TTY behavior — no Ollama instance exists in CI
  or this project's sandboxed dev environment. Verified manually
  during implementation (run `judge` against a real captured
  screenshot with Ollama running, confirm a sane response comes back)
  and noted in the PR template's Verification section, same pattern
  already used for real-TTY tests.

## Critical files

- `tools/visual-snapshot/src/judge.rs` — new module: prompt, request/
  response, `JudgeError`.
- `tools/visual-snapshot/src/encode.rs` — new `png_bytes` helper.
- `tools/visual-snapshot/src/main.rs` — `judge` subcommand,
  `--review`/`--context`/`--model` flags.
- `tools/visual-snapshot/Cargo.toml` — `ureq`, `base64` dependencies.
- `tools/visual-snapshot/README.md` — new usage section.
- `.claude/rules/development-conventions.md` — "Visual review" section
  update.

## Verification

- `cargo build --all-targets` / `cargo clippy --all-targets -- -D
  warnings` / `cargo fmt --check` — clean.
- `cargo test` — all new unit tests pass (prompt/request/response/
  encode/CLI-parsing), full existing suite unchanged elsewhere.
- Manually confirm (once, during implementation, per the
  real-external-service exemption above): with a local Ollama instance
  running and `moondream` pulled, `judge <captured-image.png>` and
  `--review` both return a coherent judgment for at least one real
  captured example screenshot — noted in the PR's Verification
  section.
