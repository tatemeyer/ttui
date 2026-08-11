//! CLI entry point: builds the requested example, drives it through a
//! script, and writes the resulting frame(s) to `--out` as a PNG or GIF.
//! See `tools/visual-snapshot/README.md` for the full command reference.

use clap::Parser as ClapParser;
use visual_snapshot::{encode, pty, script};

#[derive(ClapParser)]
struct Args {
    #[arg(long)]
    example: String,
    #[arg(long, default_value = "80x24")]
    size: String,
    #[arg(long)]
    script: std::path::PathBuf,
    #[arg(long)]
    out: std::path::PathBuf,
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
    let (cols, rows) = parse_size(&args.size)?;

    let binary = pty::build_example(&args.example)?;
    let steps = script::parse_script(&args.script)?;
    let frames = pty::run_script(&binary, rows, cols, &steps)?;

    validate_output_extension(&args.out, frames.len())?;
    if frames.len() == 1 {
        encode::write_png(&frames[0].0, &args.out)?;
    } else {
        encode::write_gif(&frames, &args.out)?;
    }

    println!("wrote {} frame(s) to {}", frames.len(), args.out.display());
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
}
