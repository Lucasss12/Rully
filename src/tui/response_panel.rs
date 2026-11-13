use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use super::{Response, SendState, view};

pub(super) fn render(frame: &mut Frame, area: Rect, focused: bool, state: &SendState, scroll: u16) {
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

    // The panel height only exists at draw time, so the scroll is clamped
    // here. One content line always renders on one row (no `.wrap()`), and a
    // trailing `\n` adds an empty rendered line — counted explicitly in
    // `total_lines`. The offset comes from app state, guarded by `max_offset`.
    let viewport_height = block.inner(area).height;
    let total = total_lines(&content);
    let offset = scroll.min(max_offset(total, viewport_height));

    frame.render_widget(
        Paragraph::new(content)
            .style(style)
            .block(block)
            .scroll((offset, 0)),
        area,
    );

    // The thumb rides the right border and only shows up once the content
    // actually overflows. `ScrollbarState` positions run in [0,
    // content_length - 1], so feeding it one "slot" per reachable offset
    // makes its maximum coincide with our clamp: the thumb then reaches the
    // exact bottom of the track when the last line is on screen.
    if total > viewport_height as usize {
        let slots = total - viewport_height as usize + 1;
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        let mut scrollbar_state = ScrollbarState::new(slots)
            .position(offset as usize)
            .viewport_content_length(1);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Lines rendered beyond the viewport: how far the content can be pushed up.
fn max_offset(total_lines: usize, viewport_height: u16) -> u16 {
    let visible = viewport_height as usize;
    u16::try_from(total_lines.saturating_sub(visible)).unwrap_or(u16::MAX)
}

/// Rendered lines, including the empty one a trailing `\n` produces.
fn total_lines(content: &str) -> usize {
    content.lines().count() + usize::from(content.ends_with('\n'))
}

fn format_body(response: &Response) -> String {
    super::content::pretty_json(&response.body).unwrap_or_else(|| response.body.clone())
}
