use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, List, ListState, Paragraph, StatefulWidget, Widget},
};

pub(super) struct LeftPanel<'a> {
    pub(super) items: &'a [String],
    pub(super) error: Option<&'a str>,
    pub(super) title: &'a str,
    pub(super) empty: &'a str,
}

impl StatefulWidget for LeftPanel<'_> {
    type State = ListState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        if let Some(error) = self.error {
            Paragraph::new(error)
                .block(Block::bordered().title(self.title))
                .render(area, buf);
            return;
        }
        if self.items.is_empty() {
            Paragraph::new(self.empty)
                .block(Block::bordered().title(self.title))
                .render(area, buf);
            return;
        }

        let position = state.selected().map_or(0, |index| index + 1);
        let label = format!("[{position} of {}]", self.items.len());

        let block = Block::bordered()
            .title(self.title)
            .title_bottom(Line::from(label).right_aligned());

        let items = self.items.iter().map(String::as_str);
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().reversed());
        StatefulWidget::render(list, area, buf, state);
    }
}
