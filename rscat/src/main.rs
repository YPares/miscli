mod app;
mod cli;
mod frequency;
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
    let pace = timing::Pace {
        clause: args.clause,
        sentence: args.sentence,
        length: args.length,
        rarity: args.rarity,
    };
    let mut reader = Reader::new(words, args.wpm, pace);

    // `--big` is a shorthand for the half-height pixel font, unless an explicit
    // non-default `--font` was requested.
    let font = match (args.big, args.font) {
        (true, ui::Font::Normal) => ui::Font::HalfHeight,
        (_, font) => font,
    };

    if args.ghost && font == ui::Font::Normal {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--ghost requires a pixel font: use --big or --font <size>",
        ));
    }

    if reader.is_empty() {
        eprintln!("rscat: no words to read");
        Ok(())
    } else {
        // Inline viewport: no alternate screen, so the rows stay in the
        // normal terminal flow instead of taking over the whole window.
        let options = TerminalOptions {
            viewport: Viewport::Inline(ui::height(font)),
        };
        let mut terminal = ratatui::init_with_options(options);
        let result = run(&mut terminal, &mut reader, font, args.ghost);
        ratatui::restore();
        result
    }
}

/// What the event loop should do after waiting for input or the next tick.
enum Control {
    /// Enough time elapsed: show the next word.
    Advance,
    /// Toggle between paused and playing.
    TogglePause,
    /// Quit the reader.
    Quit,
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    reader: &mut Reader,
    font: ui::Font,
    ghost: bool,
) -> io::Result<()> {
    let mut paused = false;

    while reader.current().is_some() {
        terminal.draw(|frame| ui::draw(frame, &*reader, paused, font, ghost))?;

        let control = if paused {
            wait_for_event()?
        } else {
            wait_for_tick(reader.tick())?
        };

        match control {
            Control::Quit => return Ok(()),
            Control::TogglePause => paused = !paused,
            Control::Advance => reader.advance(),
        }
    }
    Ok(())
}

/// Sleeps until the next word is due, handling input in the meantime.
fn wait_for_tick(tick: Duration) -> io::Result<Control> {
    let deadline = Instant::now() + tick;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(Control::Advance);
        } else if event::poll(remaining)?
            && let Some(control) = control(event::read()?)
        {
            return Ok(control);
        }
    }
}

/// Blocks until a key mapped to a [`Control`] is pressed.
fn wait_for_event() -> io::Result<Control> {
    loop {
        if let Some(control) = control(event::read()?) {
            return Ok(control);
        }
    }
}

/// Maps a terminal event to a [`Control`], if it is one we act on.
fn control(event: Event) -> Option<Control> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
            {
                Some(Control::Quit)
            } else if key.code == KeyCode::Char(' ') {
                Some(Control::TogglePause)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyEvent, KeyEventState};

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    fn release(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn space_toggles_pause_and_quit_keys_quit() {
        assert!(matches!(
            control(press(KeyCode::Char(' '), KeyModifiers::NONE)),
            Some(Control::TogglePause)
        ));
        assert!(matches!(
            control(press(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Control::Quit)
        ));
        assert!(matches!(
            control(press(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Control::Quit)
        ));
        assert!(matches!(
            control(press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Control::Quit)
        ));
    }

    #[test]
    fn unmatched_and_released_keys_are_ignored() {
        assert!(control(press(KeyCode::Char('x'), KeyModifiers::NONE)).is_none());
        assert!(control(release(KeyCode::Char(' '))).is_none());
        assert!(control(Event::Resize(80, 24)).is_none());
    }
}
