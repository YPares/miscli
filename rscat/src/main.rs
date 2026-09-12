mod app;
mod cli;
mod frequency;
mod orp;
mod text;
mod timing;
mod ui;

use std::io;
use std::time::Instant;

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

    // `--big` is a shorthand, unless an explicit non-default `--font` was requested.
    let font = match (args.big, args.font) {
        (true, ui::Font::Normal) => ui::Font::Sextant,
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
    /// Move one word back (paused only).
    Previous,
    /// Move one word forward (paused only).
    Next,
    /// Speed up by [`timing::WPM_STEP`].
    Faster,
    /// Slow down by [`timing::WPM_STEP`].
    Slower,
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
        if paused {
            terminal.draw(|frame| ui::draw(frame, &*reader, true, font, ghost))?;
            match wait_for_event()? {
                Control::Quit => return Ok(()),
                Control::TogglePause => paused = false,
                Control::Previous => reader.retreat(),
                Control::Next => reader.step_forward(),
                Control::Faster => reader.faster(timing::WPM_STEP),
                Control::Slower => reader.slower(timing::WPM_STEP),
                Control::Advance => {}
            }
        } else {
            // Keep the deadline fixed across speed changes so the current word
            // is not restarted when `+`/`-` is pressed mid-word.
            let deadline = Instant::now() + reader.tick();
            loop {
                terminal.draw(|frame| ui::draw(frame, &*reader, false, font, ghost))?;
                match wait_for_tick(deadline)? {
                    Control::Advance => {
                        reader.advance();
                        break;
                    }
                    Control::Quit => return Ok(()),
                    Control::TogglePause => {
                        paused = true;
                        break;
                    }
                    Control::Faster => reader.faster(timing::WPM_STEP),
                    Control::Slower => reader.slower(timing::WPM_STEP),
                    Control::Previous | Control::Next => {}
                }
            }
        }
    }
    Ok(())
}

/// Sleeps until `deadline`, handling input in the meantime.
fn wait_for_tick(deadline: Instant) -> io::Result<Control> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(Control::Advance);
        } else if event::poll(remaining)?
            && let Some(control) = control(event::read()?, false)
        {
            return Ok(control);
        }
    }
}

/// Blocks until a key mapped to a [`Control`] is pressed.
fn wait_for_event() -> io::Result<Control> {
    loop {
        if let Some(control) = control(event::read()?, true) {
            return Ok(control);
        }
    }
}

/// Maps a terminal event to a [`Control`], if it is one we act on.
///
/// `paused` gates the navigation keys: moving through the text only makes sense
/// while playback is stopped.
fn control(event: Event, paused: bool) -> Option<Control> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
            {
                Some(Control::Quit)
            } else if key.code == KeyCode::Char(' ') {
                Some(Control::TogglePause)
            } else if paused && key.code == KeyCode::Left {
                Some(Control::Previous)
            } else if paused && key.code == KeyCode::Right {
                Some(Control::Next)
            } else if matches!(key.code, KeyCode::Char('+') | KeyCode::Char('=')) {
                Some(Control::Faster)
            } else if key.code == KeyCode::Char('-') {
                Some(Control::Slower)
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
            control(press(KeyCode::Char(' '), KeyModifiers::NONE), false),
            Some(Control::TogglePause)
        ));
        assert!(matches!(
            control(press(KeyCode::Char('q'), KeyModifiers::NONE), false),
            Some(Control::Quit)
        ));
        assert!(matches!(
            control(press(KeyCode::Esc, KeyModifiers::NONE), false),
            Some(Control::Quit)
        ));
        assert!(matches!(
            control(press(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            Some(Control::Quit)
        ));
    }

    #[test]
    fn arrows_navigate_only_when_paused() {
        assert!(matches!(
            control(press(KeyCode::Left, KeyModifiers::NONE), true),
            Some(Control::Previous)
        ));
        assert!(matches!(
            control(press(KeyCode::Right, KeyModifiers::NONE), true),
            Some(Control::Next)
        ));
        assert!(control(press(KeyCode::Left, KeyModifiers::NONE), false).is_none());
        assert!(control(press(KeyCode::Right, KeyModifiers::NONE), false).is_none());
    }

    #[test]
    fn speed_keys_work_while_playing_and_paused() {
        for paused in [true, false] {
            assert!(matches!(
                control(press(KeyCode::Char('+'), KeyModifiers::NONE), paused),
                Some(Control::Faster)
            ));
            // `+` without shift is usually the `=` key.
            assert!(matches!(
                control(press(KeyCode::Char('='), KeyModifiers::NONE), paused),
                Some(Control::Faster)
            ));
            assert!(matches!(
                control(press(KeyCode::Char('-'), KeyModifiers::NONE), paused),
                Some(Control::Slower)
            ));
        }
    }

    #[test]
    fn unmatched_and_released_keys_are_ignored() {
        assert!(control(press(KeyCode::Char('x'), KeyModifiers::NONE), false).is_none());
        assert!(control(release(KeyCode::Char(' ')), true).is_none());
        assert!(control(Event::Resize(80, 24), true).is_none());
    }
}
