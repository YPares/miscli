use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};

use crate::app::Reader;
use crate::orp;
use crate::timing;

/// Rows needed by the UI: word, progress bar, stats.
/// Must match the vertical layout in [`draw`].
pub const HEIGHT: u16 = 3;

pub fn draw(frame: &mut Frame, reader: &Reader) {
    let [stage, bar, status] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_word(frame, reader, stage);
    render_progress(frame, reader, bar);
    render_status(frame, reader, status);
}

fn render_word(frame: &mut Frame, reader: &Reader, stage: Rect) {
    match reader.current() {
        Some(word) if stage.width > 0 && stage.height > 0 => {
            let area = Rect::new(stage.x, stage.y + stage.height / 2, stage.width, 1);
            frame.render_widget(Paragraph::new(word_line(word, stage)), area);
        }
        _ => {}
    }
}

/// Builds the word line so its pivot character lands on the horizontal center
/// of the stage, aligning with the centered gauge label and stats. The whole
/// word is clamped inside the stage.
fn word_line(word: &str, stage: Rect) -> Line<'static> {
    let (left, pivot, right) = orp::split_word(word);
    let left_width = left.chars().count() as u16;
    let word_width = left_width + 1 + right.chars().count() as u16;
    let pivot_x = stage.x + stage.width / 2;
    let max_start = stage.right().saturating_sub(word_width);
    let start = pivot_x
        .saturating_sub(left_width)
        .min(max_start)
        .max(stage.x);
    let padding = " ".repeat((start - stage.x) as usize);

    Line::from(vec![
        Span::raw(padding),
        Span::raw(left.to_string()),
        Span::styled(pivot.to_string(), Style::new().fg(Color::Red).bold()),
        Span::raw(right.to_string()),
    ])
}

fn render_progress(frame: &mut Frame, reader: &Reader, area: Rect) {
    let ratio = reader.progress_ratio().clamp(0.0, 1.0);
    let gauge = Gauge::default()
        .ratio(ratio)
        .label(format!("{:.0}%", ratio * 100.0));
    frame.render_widget(gauge, area);
}

fn render_status(frame: &mut Frame, reader: &Reader, area: Rect) {
    let position = reader.index().min(reader.len());
    let text = format!(
        "{}/{}   {} wpm   ETA {}",
        position,
        reader.len(),
        reader.wpm(),
        timing::format_eta(reader.remaining()),
    );
    frame.render_widget(Paragraph::new(text).centered().dim(), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Modifier;

    const WIDTH: u16 = 30;

    fn row(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
        (0..WIDTH)
            .map(|x| buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()))
            .collect()
    }

    #[test]
    fn renders_three_rows_with_pivot_and_status() {
        let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
        let reader = Reader::new(vec!["fox".to_string()], 60);
        terminal.draw(|frame| draw(frame, &reader)).unwrap();
        let buffer = terminal.backend().buffer();

        // Word row: pivot "o" anchored at the horizontal center (x = 15),
        // on the same axis as the centered gauge label and stats.
        assert_eq!(buffer.cell((14, 0)).unwrap().symbol(), "f");
        assert_eq!(buffer.cell((15, 0)).unwrap().symbol(), "o");
        assert_eq!(buffer.cell((16, 0)).unwrap().symbol(), "x");
        let pivot = buffer.cell((15, 0)).unwrap();
        assert!(pivot.modifier.contains(Modifier::BOLD));
        assert_eq!(pivot.fg, Color::Red);

        // Progress bar row and stats row.
        assert!(
            row(buffer, 1).contains("0%"),
            "progress row: {:?}",
            row(buffer, 1)
        );
        let status = row(buffer, 2);
        assert!(status.contains("0/1"), "status row: {status:?}");
        assert!(status.contains("60 wpm"), "status row: {status:?}");
    }
}
