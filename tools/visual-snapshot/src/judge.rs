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
    let json_string =
        serde_json::to_string(body).map_err(|e| JudgeError::Request(e.to_string()))?;
    ureq::post(OLLAMA_GENERATE_URL)
        .set("Content-Type", "application/json")
        .send_string(&json_string)
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;

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
        assert!(
            p.contains("This is supposed to show: a LAUNCH button that spawns a particle burst.")
        );
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
