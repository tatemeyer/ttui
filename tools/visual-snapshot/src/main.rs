//! CLI entry point: builds the requested example, drives it through a
//! script, and writes the resulting frame(s) to `--out` as a PNG or GIF.
//! See `tools/visual-snapshot/README.md` for the full command reference.

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
    // Keep this literal in sync with `judge::DEFAULT_MODEL` — see
    // judge.rs's own doc comment on `DEFAULT_MODEL` for why it can't be
    // referenced directly here.
    /// Ollama model name to judge with.
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

fn parse_size(s: &str) -> Result<(u16, u16), String> {
    let (cols, rows) = s
        .split_once('x')
        .ok_or_else(|| format!("expected COLSxROWS, got {s:?}"))?;
    let cols: u16 = cols.parse().map_err(|_| format!("bad cols in {s:?}"))?;
    let rows: u16 = rows.parse().map_err(|_| format!("bad rows in {s:?}"))?;
    Ok((cols, rows))
}

/// The output format `run_script`'s frame count actually produces: exactly
/// one frame is a single PNG snapshot, two or more is an animated GIF —
/// there is no third case.
fn expected_extension(frame_count: usize) -> &'static str {
    if frame_count == 1 {
        "png"
    } else {
        "gif"
    }
}

/// Confirms `out`'s file extension matches the format `run_script` is
/// actually about to write, given `frame_count`, so a script with 1+ steps
/// (which always produces 2+ frames — the initial frame plus one per step)
/// can never silently write GIF bytes to a path the caller named `.png`,
/// or vice versa. Case-insensitive so `.PNG`/`.GIF` aren't rejected.
fn validate_output_extension(out: &std::path::Path, frame_count: usize) -> Result<(), String> {
    let expected = expected_extension(frame_count);
    let actual = out.extension().and_then(|e| e.to_str()).unwrap_or("");
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    let kind = if frame_count == 1 { "a PNG" } else { "a GIF" };
    Err(format!(
        "script produces {frame_count} frame(s) ({kind}); --out must end in .{expected}, got {}",
        out.display()
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if let Some(Command::Judge {
        image,
        context,
        model,
    }) = args.command
    {
        match judge::judge_file(&image, context.as_deref(), &model) {
            Ok(verdict) => {
                println!("{verdict}");
                return Ok(());
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
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
        let last_frame = &frames
            .last()
            .expect("run_script always returns 1+ frames")
            .0;
        match encode::png_bytes(last_frame) {
            Ok(png_bytes) => {
                match judge::judge_png_bytes(&png_bytes, args.context.as_deref(), &args.model) {
                    Ok(verdict) => println!("--- judge review ---\n{verdict}"),
                    Err(e) => {
                        eprintln!("--- judge review failed (capture above is still valid) ---\n{e}")
                    }
                }
            }
            Err(e) => {
                eprintln!("--- judge review failed (capture above is still valid) ---\n{e}")
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cols_x_rows() {
        assert_eq!(parse_size("120x40"), Ok((120, 40)));
    }

    #[test]
    fn rejects_missing_separator() {
        assert!(parse_size("12040").is_err());
    }

    /// Guards finding #5 from the final-branch review: output format used
    /// to be chosen purely by frame count, with no check that `--out`'s
    /// extension agreed — so a script with any steps (always 2+ frames)
    /// silently wrote GIF bytes to a path the caller named `.png`.
    #[test]
    fn single_frame_requires_a_png_extension() {
        assert!(validate_output_extension(std::path::Path::new("out.png"), 1).is_ok());
        assert!(validate_output_extension(std::path::Path::new("out.PNG"), 1).is_ok());
        assert!(validate_output_extension(std::path::Path::new("out.gif"), 1).is_err());
    }

    #[test]
    fn multi_frame_requires_a_gif_extension() {
        assert!(validate_output_extension(std::path::Path::new("out.gif"), 4).is_ok());
        assert!(validate_output_extension(std::path::Path::new("out.GIF"), 2).is_ok());
        assert!(validate_output_extension(std::path::Path::new("out.png"), 4).is_err());
    }

    #[test]
    fn mismatch_error_names_the_frame_count_and_expected_extension() {
        let err =
            validate_output_extension(std::path::Path::new("tardis-idle.png"), 4).unwrap_err();
        assert!(
            err.contains('4'),
            "expected the frame count in the error: {err}"
        );
        assert!(
            err.contains("tardis-idle.png"),
            "expected the given path in the error: {err}"
        );
        assert!(
            err.contains(".gif"),
            "expected the required extension in the error: {err}"
        );
    }

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
        assert_eq!(args.model, judge::DEFAULT_MODEL);
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
            Some(Command::Judge {
                image,
                context,
                model,
            }) => {
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
            Some(Command::Judge { model, .. }) => assert_eq!(model, judge::DEFAULT_MODEL),
            None => panic!("expected Command::Judge to parse"),
        }
    }
}
