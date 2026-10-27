use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Padding, Paragraph},
};
use ratatui_textarea::TextArea;

use super::{Draft, METHODS, view};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    draft: Option<&Draft>,
    focused: bool,
    display_url_input: bool,
    url_input: &TextArea<'_>,
) {
    let block = Block::bordered().border_style(view::border_style(focused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [method_area, url_area] =
        Layout::horizontal([Constraint::Length(7), Constraint::Fill(1)]).areas(inner);

    let (method, url, style) = match draft {
        Some(draft) if !draft.url.is_empty() => (
            METHODS[draft.method_index],
            draft.url.as_str(),
            Style::new(),
        ),
        Some(draft) => (METHODS[draft.method_index], "Enter URL...", view::dim()),
        None => (METHODS[0], "Select a request", view::dim()),
    };

    frame.render_widget(
        Paragraph::new(method)
            .style(style)
            .block(Block::new().borders(Borders::RIGHT)),
        method_area,
    );

    if display_url_input {
        frame.render_widget(url_input, url_area);
    } else {
        frame.render_widget(
            Paragraph::new(url)
                .style(style)
                .block(Block::new().padding(Padding::left(1))),
            url_area,
        );
    }
}
