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
    #[arg(short, long, default_value_t = 300, value_parser = clap::value_parser!(u32).range(60..=2000))]
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

    /// Size of the current word.
    #[arg(short, long, value_enum, default_value = "normal")]
    pub font: Font,

    /// Shorthand for `--font half-height`.
    #[arg(short, long)]
    pub big: bool,
}
