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

    // `--big` is a shorthand, unless an explicit non-default `--font` was requested.
    let font = match (args.big, args.font) {
        (true, ui::Font::Normal) => ui::Font::Sextant,
        (_, font) => font,
    };

    if args.ghost > 0.0 && font == ui::Font::Normal {
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

/// What the event loop should do after waiting for input or a deadline.
enum Control {
    /// The deadline was reached.
    Timeout,
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

/// `ghost` is the fraction of a word's display time for which the previous
/// word's ghost is shown.
fn run(
    terminal: &mut ratatui::DefaultTerminal,
    reader: &mut Reader,
    font: ui::Font,
    ghost: f64,
) -> io::Result<()> {
    let mut paused = false;
    let ghost_on = ghost > 0.0;

    while reader.current().is_some() {
        if paused {
            terminal.draw(|frame| ui::draw(frame, &*reader, true, font, ghost_on))?;
            match wait_for_event()? {
                Control::Quit => return Ok(()),
                Control::TogglePause => paused = false,
                Control::Previous => reader.retreat(),
                Control::Next => reader.step_forward(),
                Control::Faster => reader.faster(timing::WPM_STEP),
                Control::Slower => reader.slower(timing::WPM_STEP),
                Control::Timeout => {}
            }
        } else {
            // Deadlines are fixed per word, so `+`/`-` or a pause mid-word does
            // not restart the current word's timer.
            let start = Instant::now();
            let tick = reader.tick();
            let deadline = start + tick;
            // The ghost lasts a fraction of the *previous* word's display time,
            // so a word shown longer leaves a longer trail; it can never outlast
            // the current word.
            let ghost_span = ghost_duration(reader.previous_tick(), tick, ghost);

            if !ghost_span.is_zero() {
                match play_phase(terminal, reader, font, true, start + ghost_span)? {
                    Control::Timeout => {}
                    Control::Quit => return Ok(()),
                    Control::TogglePause => {
                        paused = true;
                        continue;
                    }
                    Control::Previous | Control::Next | Control::Faster | Control::Slower => {}
                }
            }

            if ghost_span < tick {
                match play_phase(terminal, reader, font, false, deadline)? {
                    Control::Timeout => reader.advance(),
                    Control::Quit => return Ok(()),
                    Control::TogglePause => paused = true,
                    Control::Previous | Control::Next | Control::Faster | Control::Slower => {}
                }
            } else {
                // Ghost held for the whole word: no plain phase to draw.
                reader.advance();
            }
        }
    }
    Ok(())
}

/// Shows the current word (with or without its ghost) until `deadline`,
/// redrawing on speed changes. Returns the [`Control`] that ended the wait.
fn play_phase(
    terminal: &mut ratatui::DefaultTerminal,
    reader: &mut Reader,
    font: ui::Font,
    ghost: bool,
    deadline: Instant,
) -> io::Result<Control> {
    loop {
        terminal.draw(|frame| ui::draw(frame, &*reader, false, font, ghost))?;
        match wait_until(deadline)? {
            Control::Faster => reader.faster(timing::WPM_STEP),
            Control::Slower => reader.slower(timing::WPM_STEP),
            control => return Ok(control),
        }
    }
}

/// How long the previous word's ghost is shown while the current word is on
/// screen: a fraction of the *previous* word's display time, capped at the
/// current word's own display time.
fn ghost_duration(previous: Duration, current: Duration, fraction: f64) -> Duration {
    previous.mul_f64(fraction.clamp(0.0, 1.0)).min(current)
}

/// Sleeps until `deadline`, handling input in the meantime.
fn wait_until(deadline: Instant) -> io::Result<Control> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(Control::Timeout);
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
    fn ghost_duration_follows_the_previous_word_and_caps_at_the_current() {
        let previous = Duration::from_millis(1000);
        let current = Duration::from_millis(800);
        assert_eq!(ghost_duration(previous, current, 0.0), Duration::ZERO);
        assert_eq!(
            ghost_duration(previous, current, 0.25),
            Duration::from_millis(250)
        );
        assert_eq!(ghost_duration(previous, current, 1.0), current); // capped
        assert_eq!(ghost_duration(Duration::ZERO, current, 1.0), Duration::ZERO);
        // Capped by the current word when the previous one was much longer.
        let short = Duration::from_millis(200);
        assert_eq!(ghost_duration(previous, short, 0.5), short);
    }

    #[test]
    fn unmatched_and_released_keys_are_ignored() {
        assert!(control(press(KeyCode::Char('x'), KeyModifiers::NONE), false).is_none());
        assert!(control(release(KeyCode::Char(' ')), true).is_none());
        assert!(control(Event::Resize(80, 24), true).is_none());
    }
}
