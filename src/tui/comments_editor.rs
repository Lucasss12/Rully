use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Clear},
};
use ratatui_textarea::TextArea;

use super::view;

/// Draws the hidden comment editor over the interface. Comments are optional
/// metadata, so they get a modal instead of a permanent tab.
pub(super) fn render(frame: &mut Frame, area: Rect, input: &TextArea<'_>) {
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
        .title("Comments")
        .border_style(view::border_style(true));
    let inner = block.inner(popup);

    frame.render_widget(block, popup);
    frame.render_widget(input, inner);
}
