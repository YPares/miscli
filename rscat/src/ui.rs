use ratatui::Frame;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph, Widget};
use tui_big_text::{BigText, PixelSize};

use crate::app::Reader;
use crate::orp;
use crate::timing;

/// Colour the previous word's glyphs are dimmed to when ghosting.
const GHOST_COLOR: Color = Color::DarkGray;

/// How the current word is drawn.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    /// Regular single-line text (the default).
    Normal,
    /// 8×8 pixel font: 8 rows tall, 8 columns per glyph.
    Full,
    /// Pixel font, half as tall (4 rows).
    HalfHeight,
    /// Pixel font, half as wide (8 rows).
    HalfWidth,
    /// Pixel font, a quarter the area (4 rows).
    Quadrant,
    /// Pixel font, a third as tall (3 rows).
    ThirdHeight,
    /// Pixel font, a sixth the area (3 rows).
    Sextant,
    /// Pixel font, a quarter as tall (2 rows).
    QuarterHeight,
    /// Pixel font, an eighth the area (2 rows).
    Octant,
}

impl Font {
    /// Terminal rows the word occupies.
    pub const fn rows(self) -> u16 {
        match self {
            Font::Normal => 1,
            Font::Full | Font::HalfWidth => 8,
            Font::HalfHeight | Font::Quadrant => 4,
            Font::ThirdHeight | Font::Sextant => 3,
            Font::QuarterHeight | Font::Octant => 2,
        }
    }

    /// Terminal columns used by one character. The pixel font is monospaced,
    /// which is what makes pivot alignment easy.
    const fn advance(self) -> u16 {
        match self {
            Font::Normal => 1,
            Font::Full | Font::HalfHeight | Font::ThirdHeight | Font::QuarterHeight => 8,
            Font::HalfWidth | Font::Quadrant | Font::Sextant | Font::Octant => 4,
        }
    }

    /// The pixel size, or `None` for regular text.
    fn pixel_size(self) -> Option<PixelSize> {
        match self {
            Font::Normal => None,
            Font::Full => Some(PixelSize::Full),
            Font::HalfHeight => Some(PixelSize::HalfHeight),
            Font::HalfWidth => Some(PixelSize::HalfWidth),
            Font::Quadrant => Some(PixelSize::Quadrant),
            Font::ThirdHeight => Some(PixelSize::ThirdHeight),
            Font::Sextant => Some(PixelSize::Sextant),
            Font::QuarterHeight => Some(PixelSize::QuarterHeight),
            Font::Octant => Some(PixelSize::Octant),
        }
    }
}

/// Total rows needed by the UI: the word plus the stats line.
pub fn height(font: Font) -> u16 {
    font.rows() + 1
}

pub fn draw(frame: &mut Frame, reader: &Reader, paused: bool, font: Font, ghost: bool) {
    let [word, stats] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

    render_word(frame, reader, word, font, ghost);
    render_stats(frame, reader, stats, paused);
}

fn render_word(frame: &mut Frame, reader: &Reader, area: Rect, font: Font, ghost: bool) {
    match reader.current() {
        Some(word) if !area.is_empty() => match font.pixel_size() {
            None => {
                frame.render_widget(Paragraph::new(word_line(word, area, font.advance())), area);
            }
            Some(pixel_size) => {
                let current = big_buffer(word, area, font, pixel_size);
                let previous = if ghost { reader.previous() } else { None };
                match previous {
                    Some(previous) => {
                        let ghost_buffer = big_buffer(previous, area, font, pixel_size);
                        compose(frame, area, &current, Some(&ghost_buffer));
                    }
                    None => compose(frame, area, &current, None),
                }
            }
        },
        _ => {}
    }
}

/// Renders `word` as pixel text into its own buffer, so it can be composited.
fn big_buffer(word: &str, area: Rect, font: Font, pixel_size: PixelSize) -> Buffer {
    let mut buffer = Buffer::empty(area);
    BigText::builder()
        .pixel_size(pixel_size)
        .left_aligned()
        .lines(vec![word_line(word, area, font.advance())])
        .build()
        .render(area, &mut buffer);
    buffer
}

/// Copies `current` into the frame; where it has no ink, the previous word
/// (`ghost`) shows through with only its foreground dimmed, preserving the
/// glyph shapes (including partial-fill characters). Rendering through scratch
/// buffers (rather than drawing ghost-then-word) keeps the ghost where the
/// current word has no ink, since `BigText` writes the blank cells inside its
/// glyph grid.
fn compose(frame: &mut Frame, area: Rect, current: &Buffer, ghost: Option<&Buffer>) {
    let buffer = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cur = current.cell((x, y)).unwrap();
            let cell = if cur.symbol() != " " {
                cur.clone()
            } else {
                match ghost.and_then(|ghost| ghost.cell((x, y))) {
                    Some(prev) if prev.symbol() != " " => {
                        let mut cell = prev.clone();
                        cell.set_fg(GHOST_COLOR);
                        cell
                    }
                    _ => Cell::new(" "),
                }
            };
            *buffer.cell_mut((x, y)).unwrap() = cell;
        }
    }
}

/// Builds the word line with the pivot glyph at a fixed column near the center
/// of `area`. `advance` is the on-screen width of one character: 1 for regular
/// text, the pixel font's glyph width otherwise. Snapping the pivot to a
/// multiple of `advance` keeps the focal point from moving between words.
fn word_line(word: &str, area: Rect, advance: u16) -> Line<'static> {
    let (left, pivot, right) = orp::split_word(word);
    let left_chars = left.chars().count() as u16;
    let total_chars = left_chars + 1 + right.chars().count() as u16;

    let center = area.width / 2;
    let pivot_start = center.saturating_sub(advance / 2) / advance * advance;
    let max_pad = (area.width / advance).saturating_sub(total_chars);
    let pad = (pivot_start / advance)
        .saturating_sub(left_chars)
        .min(max_pad);
    let padding = " ".repeat(pad as usize);

    Line::from(vec![
        Span::raw(padding),
        Span::raw(left.to_string()),
        Span::styled(pivot.to_string(), Style::new().fg(Color::Red).bold()),
        Span::raw(right.to_string()),
    ])
}

/// Draws the stats as the label of a gauge, so the fill behind the text acts
/// as the progress bar and no separate percentage is shown.
fn render_stats(frame: &mut Frame, reader: &Reader, area: Rect, paused: bool) {
    let position = reader.index().min(reader.len());
    let marker = if paused { "PAUSED   " } else { "" };
    let text = format!(
        "{marker}{}/{}   {} wpm   ETA {}",
        position,
        reader.len(),
        reader.wpm(),
        timing::format_eta(reader.remaining()),
    );
    let ratio = reader.progress_ratio().clamp(0.0, 1.0);
    // The gauge tints the whole line with its fill colour, so force the label
    // back to the default foreground; only the fill behind it is coloured.
    let label = Span::styled(text, Style::new().fg(Color::Reset));
    let gauge = Gauge::default()
        .ratio(ratio)
        .label(label)
        .gauge_style(Style::new().fg(Color::DarkGray));
    frame.render_widget(gauge, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::Pace;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Modifier;

    const WIDTH: u16 = 30;

    fn reader(words: &[&str], wpm: u32) -> Reader {
        Reader::new(
            words.iter().map(|w| (*w).to_string()).collect(),
            wpm,
            Pace::default(),
        )
    }

    fn row(buffer: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
        (0..width)
            .map(|x| buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()))
            .collect()
    }

    /// Columns spanned by red (pivot) cells, or `None` if there are none.
    /// Leftmost column containing a red (pivot) cell, or `None`.
    ///
    /// Only the left edge is asserted because the right edge depends on the
    /// glyph's ink width (e.g. `o` and `d` end on different columns).
    fn red_start(buffer: &ratatui::buffer::Buffer, width: u16, rows: u16) -> Option<u16> {
        (0..rows)
            .flat_map(|y| {
                (0..width).filter(move |&x| buffer.cell((x, y)).unwrap().fg == Color::Red)
            })
            .min()
    }

    /// Word-row cells dimmed to the ghost colour.
    fn ghost_count(buffer: &ratatui::buffer::Buffer, width: u16, rows: u16) -> usize {
        (0..rows)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| buffer.cell((x, y)).unwrap().fg == GHOST_COLOR)
            .count()
    }

    #[test]
    fn font_heights_include_the_stats_line() {
        assert_eq!(height(Font::Normal), 2);
        assert_eq!(height(Font::HalfHeight), 5);
        assert_eq!(height(Font::Octant), 3);
    }

    #[test]
    fn renders_word_and_combined_stats_line() {
        let height = height(Font::Normal);
        let mut terminal = Terminal::new(TestBackend::new(WIDTH, height)).unwrap();
        let reader = reader(&["fox"], 60);
        terminal
            .draw(|frame| draw(frame, &reader, false, Font::Normal, false))
            .unwrap();
        let buffer = terminal.backend().buffer();

        // Word row: pivot "o" anchored at the horizontal center (x = 15).
        assert_eq!(buffer.cell((14, 0)).unwrap().symbol(), "f");
        assert_eq!(buffer.cell((15, 0)).unwrap().symbol(), "o");
        assert_eq!(buffer.cell((16, 0)).unwrap().symbol(), "x");
        let pivot = buffer.cell((15, 0)).unwrap();
        assert!(pivot.modifier.contains(Modifier::BOLD));
        assert_eq!(pivot.fg, Color::Red);

        // Combined line: stats text, and no explicit percentage.
        let stats = row(buffer, 1, WIDTH);
        assert!(stats.contains("0/1"), "stats line: {stats:?}");
        assert!(stats.contains("60 wpm"), "stats line: {stats:?}");
        assert!(!stats.contains('%'), "percentage should be gone: {stats:?}");
        assert!(!stats.contains("PAUSED"), "not paused: {stats:?}");
    }

    #[test]
    fn stats_background_fills_with_progress() {
        let height = height(Font::Normal);
        let mut terminal = Terminal::new(TestBackend::new(WIDTH, height)).unwrap();
        let mut reader = reader(&["a", "b", "c", "d"], 60);
        reader.advance();
        reader.advance();
        terminal
            .draw(|frame| draw(frame, &reader, false, Font::Normal, false))
            .unwrap();
        let buffer = terminal.backend().buffer();

        // Half progress: leftmost cell is filled, rightmost is not.
        assert_eq!(buffer.cell((0, 1)).unwrap().symbol(), "█");
        assert_eq!(buffer.cell((WIDTH - 1, 1)).unwrap().symbol(), " ");
        assert!(row(buffer, 1, WIDTH).contains("2/4"));
    }

    #[test]
    fn stats_line_marks_paused_state() {
        let height = height(Font::Normal);
        let mut terminal = Terminal::new(TestBackend::new(WIDTH, height)).unwrap();
        let reader = reader(&["fox"], 60);
        terminal
            .draw(|frame| draw(frame, &reader, true, Font::Normal, false))
            .unwrap();
        let stats = row(terminal.backend().buffer(), 1, WIDTH);
        assert!(stats.contains("PAUSED"), "stats line: {stats:?}");
    }

    #[test]
    fn big_font_pins_the_pivot_glyph_to_a_fixed_column() {
        // width 80, HalfHeight: advance 8, pivot glyph pinned starting at column 32
        // regardless of how many characters precede it.
        let font = Font::HalfHeight;
        let width = 80;
        let rows = height(font);
        for word in ["fox", "coding"] {
            let mut terminal = Terminal::new(TestBackend::new(width, rows)).unwrap();
            let reader = reader(&[word], 60);
            terminal
                .draw(|frame| draw(frame, &reader, false, font, false))
                .unwrap();
            let buffer = terminal.backend().buffer();
            assert_eq!(
                red_start(buffer, width, font.rows()),
                Some(32),
                "word {word:?}"
            );
            assert!(
                row(buffer, rows - 1, width).contains("60 wpm"),
                "stats line missing for {word:?}"
            );
        }
    }

    #[test]
    fn ghost_overlays_the_previous_word_behind_the_current_one() {
        let font = Font::HalfHeight;
        let width = 80;
        let rows = height(font);
        let word_rows = font.rows();
        // current = "coding" (pivot ink starting at column 32), previous = "ab".
        let mut reader = reader(&["ab", "coding"], 60);
        reader.advance();

        let render = |ghost: bool| -> Buffer {
            let mut terminal = Terminal::new(TestBackend::new(width, rows)).unwrap();
            terminal
                .draw(|frame| draw(frame, &reader, false, font, ghost))
                .unwrap();
            terminal.backend().buffer().clone()
        };

        let plain = render(false);
        assert_eq!(
            ghost_count(&plain, width, word_rows),
            0,
            "no ghost without --ghost"
        );
        assert_eq!(red_start(&plain, width, font.rows()), Some(32));

        let ghosted = render(true);
        assert!(
            ghost_count(&ghosted, width, word_rows) > 0,
            "the previous word should show as shade"
        );
        // The current word still wins: its pivot stays red on the same columns.
        assert_eq!(red_start(&ghosted, width, font.rows()), Some(32));
    }
}
