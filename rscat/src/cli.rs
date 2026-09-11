use std::path::PathBuf;

use clap::Parser;

/// RSVP speed-reader for plain text.
#[derive(Parser, Debug)]
#[command(name = "rscat", version, about)]
pub struct Args {
    /// File to read. If omitted, text is read from stdin.
    pub file: Option<PathBuf>,

    /// Reading speed in words per minute.
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u32).range(60..=2000))]
    pub wpm: u32,
}
