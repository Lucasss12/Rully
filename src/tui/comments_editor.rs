use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Clear, Paragraph},
};
use ratatui_textarea::TextArea;

use super::view;

/// Draws a modal text editor over the interface. Comments and environment
/// files are plain text, so they get a modal instead of a permanent tab.
///
/// `footer` is an optional hint or error line under the editor; `None` gives
/// the whole popup to the text.
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    input: &TextArea<'_>,
    title: &str,
    footer: Option<Line<'static>>,
) {
    let width = (area.width.saturating_mul(3) / 4).max(20).min(area.width);
    let height = (area.height.saturating_mul(2) / 3).max(5).min(area.height);
    let popup = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };

    frame.render_widget(Clear, popup);

    let block = Block::bordered()
        .title(title)
        .border_style(view::border_style(true));
    let inner = block.inner(popup);

    frame.render_widget(block, popup);

    match footer {
        None => frame.render_widget(input, inner),
        Some(footer) => {
            let [body, hint] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);
            frame.render_widget(input, body);
            frame.render_widget(Paragraph::new(footer), hint);
        }
    }
}
