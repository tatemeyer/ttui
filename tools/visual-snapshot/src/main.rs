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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let (cols, rows) = parse_size(&args.size)?;

    let binary = pty::build_example(&args.example)?;
    let steps = script::parse_script(&args.script)?;
    let frames = pty::run_script(&binary, rows, cols, &steps)?;

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
}
