use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Padding, Paragraph},
};

use super::{Draft, METHODS, view};

pub(super) fn render(frame: &mut Frame, area: Rect, draft: Option<&Draft>, focused: bool) {
    let block = Block::bordered().border_style(view::border_style(focused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [method_area, url_area] =
        Layout::horizontal([Constraint::Length(7), Constraint::Fill(1)]).areas(inner);

    let (method, url, style) = match draft {
        Some(draft) => (
            METHODS[draft.method_index],
            draft.url.as_str(),
            Style::new(),
        ),
        None => ("", "Select a request", view::dim()),
    };

    frame.render_widget(
        Paragraph::new(method)
            .style(style)
            .block(Block::new().borders(Borders::RIGHT)),
        method_area,
    );
    frame.render_widget(
        Paragraph::new(url)
            .style(style)
            .block(Block::new().padding(Padding::left(1))),
        url_area,
    );
}
