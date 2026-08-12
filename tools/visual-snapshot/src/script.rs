//! Parses a snapshot script — a flat JSON array of wait/key/click steps
//! that `pty::run_script` drives a spawned example through.

use serde::Deserialize;
use std::path::Path;

/// One step of a snapshot script: a real wall-clock pause, a named key
/// press, or a click, sent to the spawned example.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Step {
    /// Sleep `wait_ms` milliseconds of real time before the next step.
    Wait {
        /// Duration to sleep, in milliseconds.
        wait_ms: u64,
    },
    /// Send the named key (see `keys::encode_key`) to the spawned example.
    Key {
        /// The key's name, as written in the script (e.g. `"Right"`).
        key: String,
    },
    /// Send a left-button click at the given cell coordinates to the
    /// spawned example.
    Click {
        /// Column (0-indexed) to click.
        x: u16,
        /// Row (0-indexed) to click.
        y: u16,
    },
}

/// Failure reading or parsing a snapshot script file.
#[derive(Debug)]
pub enum ScriptError {
    /// Underlying filesystem I/O failure reading the script file.
    Io(std::io::Error),
    /// Failure parsing the file's contents as a script's JSON shape.
    Json(serde_json::Error),
}

impl From<std::io::Error> for ScriptError {
    fn from(e: std::io::Error) -> Self {
        ScriptError::Io(e)
    }
}

impl From<serde_json::Error> for ScriptError {
    fn from(e: serde_json::Error) -> Self {
        ScriptError::Json(e)
    }
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ScriptError {}

/// Reads and parses a snapshot script: a flat JSON array of `{"wait_ms": N}`,
/// `{"key": "Name"}`, and `{"x": N, "y": N}` steps.
pub fn parse_script(path: &Path) -> Result<Vec<Step>, ScriptError> {
    let contents = std::fs::read_to_string(path)?;
    let steps = serde_json::from_str(&contents)?;
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_mix_of_wait_and_key_steps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(
            &path,
            r#"[{"wait_ms":16},{"key":"Right"},{"wait_ms":150},{"key":"Enter"}]"#,
        )
        .unwrap();

        let steps = parse_script(&path).unwrap();

        assert_eq!(
            steps,
            vec![
                Step::Wait { wait_ms: 16 },
                Step::Key {
                    key: "Right".to_string()
                },
                Step::Wait { wait_ms: 150 },
                Step::Key {
                    key: "Enter".to_string()
                },
            ]
        );
    }

    #[test]
    fn empty_script_parses_to_an_empty_vec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(&path, "[]").unwrap();

        assert_eq!(parse_script(&path).unwrap(), Vec::new());
    }

    #[test]
    fn malformed_json_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(&path, "not json").unwrap();

        assert!(matches!(parse_script(&path), Err(ScriptError::Json(_))));
    }

    #[test]
    fn missing_file_is_an_error() {
        let missing = std::path::Path::new("/does/not/exist.json");
        assert!(matches!(parse_script(missing), Err(ScriptError::Io(_))));
    }

    #[test]
    fn parses_a_click_step() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(&path, r#"[{"x":10,"y":5}]"#).unwrap();

        let steps = parse_script(&path).unwrap();

        assert_eq!(steps, vec![Step::Click { x: 10, y: 5 }]);
    }

    #[test]
    fn parses_a_mix_of_wait_key_and_click_steps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.json");
        std::fs::write(&path, r#"[{"wait_ms":16},{"key":"Enter"},{"x":10,"y":5}]"#).unwrap();

        let steps = parse_script(&path).unwrap();

        assert_eq!(
            steps,
            vec![
                Step::Wait { wait_ms: 16 },
                Step::Key {
                    key: "Enter".to_string()
                },
                Step::Click { x: 10, y: 5 },
            ]
        );
    }
}
