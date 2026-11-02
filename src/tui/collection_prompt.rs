use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, Paragraph},
};

use super::{Prompt, PromptKind, view};

pub(super) fn render(frame: &mut Frame, area: Rect, prompt: &Prompt) {
    let width = (area.width.saturating_mul(2) / 3).max(40).min(area.width);
    let height = (if prompt.accepts_text() { 7 } else { 6 })
        .max(5)
        .min(area.height);
    let popup = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };

    frame.render_widget(Clear, popup);

    let block = Block::bordered()
        .title(prompt.title())
        .border_style(view::border_style(true));
    let inner = block.inner(popup);

    frame.render_widget(block, popup);

    let [body, hint] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);

    match &prompt.kind {
        PromptKind::DeleteCollection { name } => {
            frame.render_widget(
                Paragraph::new(format!("Delete collection \"{name}\"?")).centered(),
                body,
            );
        }
        PromptKind::DeleteRequest { name, .. } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Delete request \"{}\"?",
                    super::request_display_name(name)
                ))
                .centered(),
                body,
            );
        }
        PromptKind::DeleteEnvironment { name } => {
            frame.render_widget(
                Paragraph::new(format!("Delete environment \"{name}\"?")).centered(),
                body,
            );
        }
        PromptKind::NewCollection
        | PromptKind::RenameCollection { .. }
        | PromptKind::NewRequest { .. }
        | PromptKind::RenameRequest { .. }
        | PromptKind::NewEnvironment
        | PromptKind::RenameEnvironment { .. } => {
            frame.render_widget(&prompt.input, body);
        }
    }

    let style = if prompt.error.is_some() {
        Style::new().fg(Color::Red)
    } else {
        view::dim()
    };
    let message = prompt.error.as_deref().unwrap_or_else(|| prompt.hint());

    frame.render_widget(Paragraph::new(message).style(style), hint);
}
