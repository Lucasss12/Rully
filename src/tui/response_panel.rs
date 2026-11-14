use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use super::{Response, SendState, view};

/// The response text prepared for display: built once when the response
/// arrives, then blitted by every frame. Building it is the expensive part
/// (JSON pretty-printing dominates), so it stays out of the draw path. The
/// content is kept line-split because `Text::from` on a huge string would
/// re-split it on every frame.
#[derive(Debug)]
pub(super) struct ResponseText {
    pub(super) lines: Vec<String>,
}

/// Assembles status, duration, headers and body into the display text,
/// split on newlines (a trailing `\n` yields a final empty line, as
/// rendered).
pub(super) fn build_response_text(response: &Response) -> ResponseText {
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

    let content = format!(
        "{} {}\n{} ms\n\n{}\n\n{}",
        response.status.as_u16(),
        response.status.canonical_reason().unwrap_or(""),
        response.duration_ms(),
        headers.join("\n"),
        super::content::pretty_json(&response.body).unwrap_or_else(|| response.body.clone())
    );

    ResponseText {
        lines: content.split('\n').map(str::to_string).collect(),
    }
}

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    focused: bool,
    state: &SendState,
    prepared: Option<&ResponseText>,
    scroll: u16,
) {
    let block = Block::bordered()
        .title("Response")
        .border_style(view::border_style(focused));
    let viewport_height = block.inner(area).height;

    let (content, style, total, offset) = match state {
        SendState::Idle => (String::from("No response yet"), view::dim(), 1, 0),
        SendState::Sending => (String::from("Sending…"), view::dim(), 1, 0),
        SendState::Done(response) => {
            let style = Style::default();
            // The normal path blits the window of the prepared lines;
            // a direct `send` assignment bypassing `apply_response` builds
            // its own lines here, at the usual cost.
            let lines: &[String] = match prepared {
                Some(text) => &text.lines,
                None => &build_response_text(response).lines,
            };
            let total = lines.len();
            let offset = scroll.min(max_offset(total, viewport_height));

            let start = (offset as usize).min(total);
            let end = (start + viewport_height as usize).min(total);
            let window = lines[start..end].join("\n");

            (window, style, total, offset)
        }
        SendState::Failed(message) => (message.clone(), Style::new().fg(Color::Red), 1, 0),
    };

    // The content handed to the paragraph is exactly the visible window, so
    // no scroll is applied at the widget level; `offset` only drives the
    // scrollbar below.
    frame.render_widget(
        Paragraph::new(content.as_str()).style(style).block(block),
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
