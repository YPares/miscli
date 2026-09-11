use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};

use crate::app::Reader;
use crate::orp;
use crate::timing;

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

/// Builds the word line so its pivot character lands on the stage's pivot
/// column (one third in), clamping the whole word inside the stage.
fn word_line(word: &str, stage: Rect) -> Line<'static> {
    let (left, pivot, right) = orp::split_word(word);
    let left_width = left.chars().count() as u16;
    let word_width = left_width + 1 + right.chars().count() as u16;
    let pivot_x = stage.x + stage.width / 3;
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
