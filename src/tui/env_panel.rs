use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, List, ListState, Paragraph, StatefulWidget, Widget},
};

use super::view;

/// The environments strip below the tree: every `*.env` name, with an accent
/// dot on the active one. Always rendered — seeing which environment is
/// selected is the whole point of the section.
pub(super) struct EnvPanel<'a> {
    pub(super) environments: &'a [String],
    pub(super) active: Option<&'a str>,
    pub(super) focused: bool,
}

impl StatefulWidget for EnvPanel<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let block = Block::bordered()
            .title("Environments")
            .title_style(view::title())
            .border_style(view::border_style(self.focused));

        if self.environments.is_empty() {
            Paragraph::new("No environments found")
                .style(view::dim())
                .block(block)
                .render(area, buf);
            return;
        }

        let items = self.environments.iter().map(|name| {
            if Some(name.as_str()) == self.active {
                Line::from(vec![
                    ratatui::text::Span::styled("●", view::accent()),
                    ratatui::text::Span::raw(format!(" {name}")),
                ])
            } else {
                Line::from(format!("  {name}"))
            }
        });

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().reversed());

        StatefulWidget::render(list, area, buf, state);
    }
}
