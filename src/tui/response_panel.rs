use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Paragraph},
};

use super::view;

pub(super) fn render(frame: &mut Frame, area: Rect, focused: bool) {
    let block = Block::bordered()
        .title("Response")
        .border_style(view::border_style(focused));

    frame.render_widget(
        Paragraph::new("No response yet")
            .style(view::dim())
            .block(block),
        area,
    );
}
