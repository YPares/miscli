mod app;
mod cli;
mod orp;
mod text;
mod timing;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{TerminalOptions, Viewport};

use crate::app::Reader;
use crate::cli::Args;

fn main() -> io::Result<()> {
    let args = Args::parse();
    let words = text::load_words(args.file.as_deref())?;
    let mut reader = Reader::new(words, args.wpm);

    if reader.is_empty() {
        eprintln!("rscat: no words to read");
        Ok(())
    } else {
        // Inline viewport: no alternate screen, so the three rows stay in the
        // normal terminal flow instead of taking over the whole window.
        let options = TerminalOptions {
            viewport: Viewport::Inline(ui::HEIGHT),
        };
        let mut terminal = ratatui::init_with_options(options);
        let result = run(&mut terminal, &mut reader);
        ratatui::restore();
        result
    }
}

fn run(terminal: &mut ratatui::DefaultTerminal, reader: &mut Reader) -> io::Result<()> {
    let tick = Duration::from_millis(timing::ms_per_word(reader.wpm()));

    while reader.current().is_some() {
        terminal.draw(|frame| ui::draw(frame, &*reader))?;
        if wait_for_tick(tick)? {
            return Ok(());
        } else {
            reader.advance();
        }
    }
    Ok(())
}

/// Sleeps until the next word is due, aborting early if the user quits.
/// Returns `true` when a quit key was pressed.
fn wait_for_tick(tick: Duration) -> io::Result<bool> {
    let deadline = Instant::now() + tick;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(false);
        } else if event::poll(remaining)? && is_quit(event::read()?) {
            return Ok(true);
        }
    }
}

fn is_quit(event: Event) -> bool {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        }
        _ => false,
    }
}
