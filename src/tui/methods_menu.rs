use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Clear, List, ListState},
};

pub(super) fn render(frame: &mut Frame, area: Rect, methods: &[&str], index: usize) {
    let mut state = ListState::default();
    state.select(Some(index));

    let list = List::new(methods.iter().copied())
        .block(Block::bordered())
        .highlight_style(Style::new().reversed());

    frame.render_widget(Clear, area);
    frame.render_stateful_widget(list, area, &mut state);
}
