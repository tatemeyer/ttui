# Local Vision Judge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an on-demand, local vision-model judgment step to
`tools/visual-snapshot` — a `judge` subcommand and a `--review` flag on
the existing capture command — that asks a local Ollama instance
whether a captured screenshot looks broken.

**Architecture:** A new `judge` module in `tools/visual-snapshot`
separates pure logic (prompt text, request-body construction, response
parsing — all unit-testable) from the one function that performs actual
network I/O against Ollama's local HTTP API
(`http://localhost:11434/api/generate`). The CLI gains a `judge`
subcommand (judges an existing PNG file) and `--review`/`--context`/
`--model` flags on the default capture path (judges the just-captured
final frame in-memory, no disk round-trip). Judging is always advisory:
a judge failure never changes capture's success/exit code, and none of
this is ever invoked from CI.

**Tech Stack:** Rust, `ureq` (blocking HTTP client, new dependency of
`tools/visual-snapshot` only), `base64` (new dependency, same scope),
`serde`/`serde_json` (already a dependency), Ollama running locally.

## Global Constraints

- **`coding`-tagged, TDD mandatory**, with one narrow, named exception:
  the literal network round-trip to a real Ollama instance
  (`judge::send_request`'s actual HTTP call) is real-external-service
  exempt — no Ollama instance exists in CI or this project's sandboxed
  dev environment. Everything else (prompt construction, request-body
  shape, response parsing, the new `encode::png_bytes` helper, CLI arg
  parsing) is fully TDD-covered; only that one function's live behavior
  is verified manually, once, during implementation, and noted in the
  PR's Verification section.
- **`ureq` and `base64` are dependencies of `tools/visual-snapshot`
  only** — never added to the root `ttui` library crate's
  `Cargo.toml`, matching the existing isolation already established for
  `portable-pty`.
- **Never wired into CI.** No `.github/workflows/` changes anywhere in
  this plan. This tool is never a required check and never gates a
  build or merge.
- **Never a replacement for the mandatory human/reviewer-subagent
  visual review.** A judge failure (Ollama unreachable, model not
  pulled, malformed response) must never change the exit code or
  success of the capture step it's attached to — capture and judge are
  independent outcomes.
- **No golden-image diffing, no stored reference images.** Judgment is
  standalone per screenshot only.
- **No bundling, installing, or auto-pulling of Ollama or any model.**
  The tool assumes both are already present and fails with a clear
  message if not.

---

### Task 1: `judge` module

**Files:**
- Create: `tools/visual-snapshot/src/judge.rs`
- Modify: `tools/visual-snapshot/src/lib.rs`
- Modify: `tools/visual-snapshot/Cargo.toml`

**Interfaces:**
- Produces: `judge::DEFAULT_MODEL: &str` (= `"moondream"`),
  `judge::JudgeError` (enum: `Io(std::io::Error)`, `Request(String)`,
  `Parse(String)`), `judge::build_prompt(context: Option<&str>) ->
  String`, `judge::judge_png_bytes(image_bytes: &[u8], context:
  Option<&str>, model: &str) -> Result<String, JudgeError>`,
  `judge::judge_file(path: &std::path::Path, context: Option<&str>,
  model: &str) -> Result<String, JudgeError>` — all consumed by Task 2.

- [ ] **Step 1: Write the failing tests**

Create `tools/visual-snapshot/src/judge.rs` with just this test module
at the bottom (the rest of the file — the actual implementation — comes
in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    #[test]
    fn default_model_is_moondream() {
        assert_eq!(DEFAULT_MODEL, "moondream");
    }

    #[test]
    fn build_prompt_without_context_omits_the_context_sentence() {
        let p = build_prompt(None);
        assert!(p.contains("You are reviewing a screenshot"));
        assert!(!p.contains("This is supposed to show"));
    }

    #[test]
    fn build_prompt_with_context_includes_it() {
        let p = build_prompt(Some("a LAUNCH button that spawns a particle burst"));
        assert!(p.contains(
            "This is supposed to show: a LAUNCH button that spawns a particle burst."
        ));
    }

    #[test]
    fn build_request_body_has_expected_shape() {
        let body = build_request_body(b"fake-png-bytes", Some("ctx"), "moondream");
        assert_eq!(body["model"], "moondream");
        assert_eq!(body["stream"], false);
        let images = body["images"].as_array().unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0], STANDARD.encode(b"fake-png-bytes"));
        assert!(body["prompt"].as_str().unwrap().contains("ctx"));
    }

    #[test]
    fn parse_response_body_extracts_the_response_field() {
        let json = r#"{"model":"moondream","created_at":"2026-08-12T00:00:00Z","response":"LOOKS OK. Nothing unusual.","done":true}"#;
        let text = parse_response_body(json).unwrap();
        assert_eq!(text, "LOOKS OK. Nothing unusual.");
    }

    #[test]
    fn parse_response_body_rejects_malformed_json() {
        let err = parse_response_body("not json").unwrap_err();
        assert!(matches!(err, JudgeError::Parse(_)));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package visual-snapshot --lib judge::`
Expected: FAIL to compile — `judge.rs` doesn't declare any of
`DEFAULT_MODEL`, `build_prompt`, `build_request_body`,
`parse_response_body`, or `JudgeError` yet, and `tools/visual-snapshot`
doesn't yet have `base64` as a dependency.

- [ ] **Step 3: Add the implementation**

Add these dependencies to `tools/visual-snapshot/Cargo.toml`'s
`[dependencies]` section (next to the existing `serde_json = "1"`
line):

```toml
ureq = "2"
base64 = "0.22"
```

Insert this above the `#[cfg(test)]` block already written in Step 1,
at the top of `tools/visual-snapshot/src/judge.rs`:

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
/// available. Kept in sync with the CLI's `default_value = "moondream"`
/// literals (Task 2) by the `default_model_is_moondream` test above,
/// since clap's derive macro needs a literal there, not this constant
/// directly.
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
/// glue with no branching logic of its own — real-external-service
/// exempt per this plan's Global Constraints; verified manually in
/// Task 4, not unit-tested here.
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

Add `pub mod judge;` to `tools/visual-snapshot/src/lib.rs`, in
alphabetical order with the existing `pub mod` lines (it goes between
`pub mod glyph;` and `pub mod keys;`):

```rust
pub mod color;
pub mod encode;
pub mod glyph;
pub mod judge;
pub mod keys;
pub mod pty;
pub mod render;
pub mod script;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package visual-snapshot --lib judge::`
Expected: all 6 new tests PASS.

- [ ] **Step 5: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 6: Full workspace test**

Run: `cargo test --workspace`
Expected: full suite green, no regressions elsewhere.

- [ ] **Step 7: Commit**

```bash
git add tools/visual-snapshot/src/judge.rs tools/visual-snapshot/src/lib.rs \
        tools/visual-snapshot/Cargo.toml
git commit -m "feat(visual-snapshot): add judge module for local Ollama vision review

Pure prompt/request/response logic is fully unit-tested; the actual
HTTP round-trip to a real Ollama instance is real-external-service
exempt (no Ollama in CI or this sandboxed dev environment), verified
manually once during implementation instead."
```

---

### Task 2: CLI integration

**Files:**
- Modify: `tools/visual-snapshot/src/encode.rs`
- Modify: `tools/visual-snapshot/src/main.rs`

**Interfaces:**
- Consumes: `judge::DEFAULT_MODEL`, `judge::judge_png_bytes`,
  `judge::judge_file` (Task 1).
- Produces: `encode::png_bytes(img: &RgbaImage) -> Result<Vec<u8>,
  EncodeError>` — used only within this task's own `main.rs` changes,
  not consumed by any later task.

- [ ] **Step 1: Write the failing test for `encode::png_bytes`**

Add to `tools/visual-snapshot/src/encode.rs`'s existing `#[cfg(test)]
mod tests` block (below the existing `write_gif_round_trips_frame_count`
test, reusing the existing `solid` helper already in that block):

```rust
    #[test]
    fn png_bytes_round_trips_dimensions_and_pixels() {
        let img = solid(4, 2, Rgba([10, 20, 30, 255]));

        let bytes = png_bytes(&img).unwrap();

        let reopened = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(reopened.dimensions(), (4, 2));
        assert_eq!(*reopened.get_pixel(0, 0), Rgba([10, 20, 30, 255]));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package visual-snapshot --lib encode::`
Expected: FAIL to compile — `png_bytes` doesn't exist yet.

- [ ] **Step 3: Add `png_bytes` to `encode.rs`**

Add this function to `tools/visual-snapshot/src/encode.rs`, directly
below the existing `write_png` function:

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

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package visual-snapshot --lib encode::`
Expected: all `encode::` tests PASS, including the new one.

- [ ] **Step 5: Write the failing CLI-parsing tests**

Add to `tools/visual-snapshot/src/main.rs`'s existing `#[cfg(test)] mod
tests` block, below `mismatch_error_names_the_frame_count_and_expected_extension`:

```rust
    #[test]
    fn no_subcommand_parses_as_a_plain_capture_with_review_off_by_default() {
        let args = Args::try_parse_from([
            "visual-snapshot",
            "--example",
            "tardis",
            "--script",
            "s.json",
            "--out",
            "o.gif",
        ])
        .unwrap();
        assert!(args.command.is_none());
        assert_eq!(args.example.as_deref(), Some("tardis"));
        assert!(!args.review);
        assert_eq!(args.model, "moondream");
    }

    #[test]
    fn review_flag_and_context_parse_on_the_capture_path() {
        let args = Args::try_parse_from([
            "visual-snapshot",
            "--example",
            "tardis",
            "--script",
            "s.json",
            "--out",
            "o.gif",
            "--review",
            "--context",
            "a blue box",
            "--model",
            "llava",
        ])
        .unwrap();
        assert!(args.review);
        assert_eq!(args.context.as_deref(), Some("a blue box"));
        assert_eq!(args.model, "llava");
    }

    #[test]
    fn judge_subcommand_parses_the_image_path_and_flags() {
        let args = Args::try_parse_from([
            "visual-snapshot",
            "judge",
            "capture.png",
            "--context",
            "a red dial",
            "--model",
            "llava",
        ])
        .unwrap();
        match args.command {
            Some(Command::Judge { image, context, model }) => {
                assert_eq!(image, std::path::PathBuf::from("capture.png"));
                assert_eq!(context.as_deref(), Some("a red dial"));
                assert_eq!(model, "llava");
            }
            None => panic!("expected Command::Judge to parse"),
        }
    }

    #[test]
    fn judge_subcommand_defaults_model_to_moondream() {
        let args = Args::try_parse_from(["visual-snapshot", "judge", "capture.png"]).unwrap();
        match args.command {
            Some(Command::Judge { model, .. }) => assert_eq!(model, "moondream"),
            None => panic!("expected Command::Judge to parse"),
        }
    }
```

No new imports are needed in the test module itself: it already has
`use super::*;` (matching the existing pattern in this file), which
picks up `Args`, `Command`, and the `ClapParser`-aliased `Parser` trait
(needed for `.try_parse_from`) from `main.rs`'s top-level `use`
declarations once Step 7 below updates them.

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test --package visual-snapshot --bin visual-snapshot`
Expected: FAIL to compile — `Args` doesn't have a `command` field,
`review`/`context`/`model` fields, or a `Command` enum yet, and
`example`/`script`/`out` are still non-`Optional`.

- [ ] **Step 7: Restructure `main.rs`**

Replace the entire top of `tools/visual-snapshot/src/main.rs`, from the
`use` lines through the end of the `Args` struct, with:

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
    /// Ollama model name to judge with. Keep this literal in sync with
    /// `judge::DEFAULT_MODEL` — see judge.rs's own doc comment on
    /// `DEFAULT_MODEL` for why it can't be referenced directly here.
    #[arg(long, default_value = "moondream")]
    model: String,
}

#[derive(Subcommand)]
enum Command {
    /// Ask a local Ollama vision model to judge an already-captured screenshot.
    Judge {
        /// Path to a PNG captured by this tool (or any PNG).
        image: std::path::PathBuf,
        /// Description of what the image is supposed to show.
        #[arg(long)]
        context: Option<String>,
        /// Ollama model name to judge with.
        #[arg(long, default_value = "moondream")]
        model: String,
    },
}
```

Then replace the body of `fn main()` with:

```rust
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

`parse_size`, `expected_extension`, and `validate_output_extension`
(the free functions below `main`) are unchanged — do not modify them.

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test --package visual-snapshot --bin visual-snapshot`
Expected: all tests PASS, including the 4 new CLI-parsing tests and all
5 pre-existing tests in that module
(`parses_cols_x_rows`, `rejects_missing_separator`,
`single_frame_requires_a_png_extension`,
`multi_frame_requires_a_gif_extension`,
`mismatch_error_names_the_frame_count_and_expected_extension`).

- [ ] **Step 9: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` — clean.
Run: `cargo fmt --check` — clean.

- [ ] **Step 10: Full workspace test**

Run: `cargo test --workspace`
Expected: full suite green, no regressions elsewhere.

- [ ] **Step 11: Commit**

```bash
git add tools/visual-snapshot/src/encode.rs tools/visual-snapshot/src/main.rs
git commit -m "feat(visual-snapshot): add judge subcommand and --review flag

Existing flat 'cargo run -p visual-snapshot -- --example ...' capture
invocation keeps working unchanged (command defaults to None). A judge
failure never changes capture's own success/exit code — the two are
independent outcomes, since this is advisory-only and never a gate."
```

---

### Task 3: Documentation

**Files:**
- Modify: `tools/visual-snapshot/README.md`
- Modify: `.claude/rules/development-conventions.md`

**Interfaces:** none.

- [ ] **Step 1: Add a "Judging a screenshot" section to the README**

In `tools/visual-snapshot/README.md`, insert a new section immediately
after the existing "## Output format" section and before "## Known
glyph-coverage limitation":

```markdown
## Judging a screenshot

An optional, on-demand, local vision-model judgment step — advisory
only, never wired into CI, never a replacement for the mandatory human/
reviewer-subagent visual review required before merge
(`.claude/rules/development-conventions.md`'s "Visual review" section).
Useful for fast iteration while developing an example, or as a second
opinion alongside a full review.

**Prerequisites:** [Ollama](https://ollama.com) installed and running
locally, with a vision-capable model pulled:

```
ollama pull moondream
```

**Judge an already-captured screenshot:**

```
cargo run -p visual-snapshot -- judge <path.png> [--context "description"] [--model <name>]
```

**Judge immediately after capturing** (judges the final frame — for a
multi-step script, that's the end state after all steps run):

```
cargo run -p visual-snapshot -- --example <name> --script <path.json> --out <path> --review [--context "description"]
```

- `--context "description"` — tells the model what the screenshot is
  supposed to show. Without it, the model can only catch gross
  corruption (garbled glyphs, overlapping text) — it has no notion of
  what "correct" means for a specific example otherwise.
- `--model <name>` — Ollama model to use. Defaults to `moondream`
  (small, CPU-friendly). Override with a more capable model (e.g.
  `llava`) if you have a GPU.
- A judge failure (Ollama unreachable, model not pulled, malformed
  response) is printed to stderr and never changes `--review`'s
  capture success or exit code — judging and capturing are independent
  outcomes.

The fixed prompt sent to the model: "You are reviewing a screenshot of
a terminal UI rendered by an automated test tool. [This is supposed to
show: `{context}`.] Look for: garbled or missing glyphs, broken layout
(overlapping text, content cut off unexpectedly), or anything that
looks visually wrong. Respond with a brief verdict (LOOKS OK / POSSIBLE
ISSUE) followed by 1-3 sentences of reasoning."
```

- [ ] **Step 2: Update `development-conventions.md`'s "Visual review" section**

In `.claude/rules/development-conventions.md`, find the "Visual review"
section's closing sentence: "Record which snapshots were reviewed in
the PR template's existing freeform Verification section
(`.claude/templates/github/PULL_REQUEST_TEMPLATE.md`), the same pattern
already used for real-TTY test results below." Add this new paragraph
immediately after it, before the following `## Commit conventions`
heading (additive only — do not alter the existing paragraphs above
it):

```markdown
**Optional local vision-model second opinion:** `tools/visual-snapshot`
also has a `judge` subcommand and a `--review` flag (see its README's
"Judging a screenshot" section) that ask a local Ollama instance for a
vision-model judgment on a captured screenshot. This is a fast,
optional aid for iterating on an example or getting a second opinion —
it is never a substitute for the mandatory review above, never wired
into CI, and never gates a build or merge. Its live HTTP call to a real
Ollama instance is real-TTY-style exempt from automated testing (no
Ollama instance exists in CI or this project's sandboxed dev
environment) — verified manually, the same pattern already used for
real-TTY tests above, and noted in the PR template's Verification
section when used.
```

- [ ] **Step 3: Commit**

```bash
git add tools/visual-snapshot/README.md .claude/rules/development-conventions.md
git commit -m "docs(visual-snapshot): document the judge subcommand and --review flag

Additive to development-conventions.md's Visual review section — the
existing mandatory-review requirement is unchanged; this documents a
new optional, non-blocking local aid alongside it."
```

---

### Task 4: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Build every target**

Run: `cargo build --all-targets`
Expected: succeeds.

- [ ] **Step 2: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 3: Full test suite**

Run: `cargo test --workspace`
Expected: full suite green — includes all of Task 1's `judge::` tests,
Task 2's `encode::png_bytes` test and 4 new CLI-parsing tests, and
everything else unchanged.

- [ ] **Step 4: Manual verification of the real-external-service-exempt path**

This is the one path this plan's Global Constraints exempt from
automated testing — verify it for real, once:

1. Confirm Ollama is installed and running locally
   (`ollama --version`; start it if needed).
2. `ollama pull moondream` if not already pulled.
3. Capture a real screenshot of any existing example, e.g.:
   ```
   cargo run -p visual-snapshot -- --example tardis --script <(echo '[]') --out /tmp/tardis.png
   ```
   (or any valid empty/simple script file — the point is producing one
   real PNG to judge.)
4. Run `cargo run -p visual-snapshot -- judge /tmp/tardis.png --context "a TARDIS console screen"`
   and confirm a coherent judgment is printed (a verdict line plus 1-3
   sentences of reasoning — not an error, not empty output).
5. Re-run the same capture with `--review` added instead, and confirm
   the same kind of judgment appears after the "wrote N frame(s)..."
   line.
6. Note the result of this manual check in the PR's Verification
   section (per this plan's Global Constraints) — this is the concrete
   proof the real Ollama round-trip works, not just its unit-tested
   request/response logic in isolation.

If Ollama is not available in this environment to actually perform this
check, note that explicitly in the PR's Verification section (per the
same pattern already established for glyph-coverage gaps in visual
review) rather than skipping the note entirely — do not claim the
manual check passed if it wasn't actually run.

## Final verification (whole plan)

- [ ] `cargo build --all-targets` succeeds.
- [ ] `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` — clean.
- [ ] `cargo test --workspace` — full suite green, including all new
      `judge::`/`encode::png_bytes`/CLI-parsing tests.
- [ ] The PR's Verification section states plainly whether the manual
      real-Ollama check (Task 4 Step 4) was performed, and its result.
- [ ] Per `.claude/rules/git-github-standards.md`: open a PR from this
      Arc's worktree branch to `main`, wait for all four required
      checks green, squash-merge, then remove the worktree via
      `ExitWorktree` (per the documented squash-merge resolution:
      verify via `gh pr view --json state,mergedAt,mergeCommit`, then
      retry with `discard_changes: true` if the tool's own ancestry
      check false-positives).
