use std::path::PathBuf;

use clap::Parser;

use crate::timing;
use crate::ui::Font;

/// RSVP speed-reader for plain text.
#[derive(Parser, Debug)]
#[command(name = "rscat", version, about)]
pub struct Args {
    /// File to read. If omitted, text is read from stdin.
    pub file: Option<PathBuf>,

    /// Reading speed in words per minute.
    #[arg(short, long, default_value_t = 300, value_parser = clap::value_parser!(u32).range(timing::MIN_WPM as i64..=timing::MAX_WPM as i64))]
    pub wpm: u32,

    /// Display time for words ending in `,` `;` `:` as a multiple of the plain
    /// per-word time.
    #[arg(short, long, default_value_t = timing::DEFAULT_CLAUSE_PAUSE)]
    pub clause: f64,

    /// Display time for words ending in `.` `?` `!` as a multiple of the plain
    /// per-word time.
    #[arg(short, long, default_value_t = timing::DEFAULT_SENTENCE_PAUSE)]
    pub sentence: f64,

    /// Extra display time proportional to `sqrt(word length)`, matching
    /// pasky/speedread's `$lentime`. 0 disables it.
    #[arg(short, long, default_value_t = timing::DEFAULT_LENGTH_FACTOR)]
    pub length: f64,

    /// Extra display time for words that are rare within the text, counted
    /// relative to its most frequent word. 0 disables it.
    #[arg(short, long, default_value_t = timing::DEFAULT_RARITY)]
    pub rarity: f64,

    /// Size of the current word.
    #[arg(short, long, value_enum, default_value = "normal")]
    pub font: Font,

    /// Shorthand for `--font sextant`.
    #[arg(short, long)]
    pub big: bool,

    /// Overlay the previous word behind the current one for this fraction
    /// (0..1) of the word's display time; 0 disables it. Requires a pixel font.
    #[arg(long, default_value_t = 0.0, value_parser = parse_fraction)]
    pub ghost: f64,
}

/// Parses a fraction in `0.0..=1.0` for `--ghost`.
fn parse_fraction(value: &str) -> Result<f64, String> {
    let fraction: f64 = value
        .parse()
        .map_err(|_| format!("`{value}` is not a number"))?;
    if (0.0..=1.0).contains(&fraction) {
        Ok(fraction)
    } else {
        Err("must be between 0 and 1".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fraction_accepts_only_the_unit_interval() {
        assert_eq!(parse_fraction("0"), Ok(0.0));
        assert_eq!(parse_fraction("0.5"), Ok(0.5));
        assert_eq!(parse_fraction("1"), Ok(1.0));
        assert!(parse_fraction("-0.1").is_err());
        assert!(parse_fraction("1.5").is_err());
        assert!(parse_fraction("half").is_err());
    }
}
