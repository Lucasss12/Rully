use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph},
};

use super::{Response, SendState, view};

pub(super) fn render(frame: &mut Frame, area: Rect, focused: bool, state: &SendState) {
    let block = Block::bordered()
        .title("Response")
        .border_style(view::border_style(focused));

    let (content, style) = match state {
        SendState::Idle => ("No response yet".to_string(), view::dim()),
        SendState::Sending => ("Sending…".to_string(), view::dim()),
        SendState::Done(response) => {
            let headers: Vec<String> = response
                .headers
                .iter()
                .map(|(name, value)| {
                    format!(
                        "{}: {}",
                        name.as_str(),
                        value.to_str().unwrap_or("<non-utf8>")
                    )
                })
                .collect();

            let text = format!(
                "{} {}\n{} ms\n\n{}\n\n{}",
                response.status.as_u16(),
                response.status.canonical_reason().unwrap_or(""),
                response.duration_ms(),
                headers.join("\n"),
                format_body(response)
            );
            (text, Style::default())
        }
        SendState::Failed(message) => (message.clone(), Style::new().fg(Color::Red)),
    };

    frame.render_widget(Paragraph::new(content).style(style).block(block), area);
}

fn format_body(response: &Response) -> String {
    crate::output::pretty_print_json(&response.body).unwrap_or_else(|| response.body.clone())
}
