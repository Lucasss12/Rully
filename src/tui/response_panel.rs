use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs},
};

use super::{Response, SendState, view};

/// The two response sections, cycled with the arrows like the request tabs.
/// Body first: it is what a send is about, Headers are one key away.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ResponseTab {
    Body,
    Headers,
}

impl ResponseTab {
    pub const ALL: [Self; 2] = [Self::Body, Self::Headers];

    fn index(self) -> usize {
        match self {
            ResponseTab::Body => 0,
            ResponseTab::Headers => 1,
        }
    }

    pub(super) fn switch_tabs(self, forward: bool) -> Self {
        let len = Self::ALL.len();
        let index = if forward {
            (self.index() + 1) % len
        } else {
            (self.index() + len - 1) % len
        };
        Self::ALL[index]
    }
}

/// The response text prepared for display: built once when the response
/// arrives, then blitted by every frame. Building it is the expensive part
/// (JSON pretty-printing dominates), so it stays out of the draw path. The
/// sections are line-split because `Text::from` on a huge string would
/// re-split it on every frame.
#[derive(Debug)]
pub(super) struct ResponseText {
    /// Status (colored by class), duration and size, right of the tabs row.
    pub(super) status_line: Line<'static>,
    pub(super) headers: Vec<String>,
    pub(super) body: Vec<String>,
}

/// Assembles status, duration, headers and body into display sections,
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

    let body = super::content::pretty_json(&response.body).unwrap_or_else(|| response.body.clone());

    ResponseText {
        status_line: Line::from(vec![
            Span::styled(
                format!("{} ", response.status.as_u16()),
                view::status_style(response.status.as_u16()),
            ),
            Span::raw(format!(
                "{} · {} ms · {}",
                response.status.canonical_reason().unwrap_or(""),
                response.duration_ms(),
                format_size(response.size),
            )),
        ]),
        headers: if headers.is_empty() {
            vec!["(no headers)".to_string()]
        } else {
            headers
        },
        body: if body.is_empty() {
            vec!["(no body)".to_string()]
        } else {
            body.split('\n').map(str::to_string).collect()
        },
    }
}

/// Human-readable size, the same shaping the CLI used to print.
fn format_size(bytes: usize) -> String {
    let units = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} {}", units[unit])
    } else {
        format!("{size:.1} {}", units[unit])
    }
}

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    focused: bool,
    state: &SendState,
    tab: ResponseTab,
    prepared: Option<&ResponseText>,
    scroll: u16,
) {
    let block = Block::bordered()
        .title("Response")
        .title_style(view::title())
        .border_style(view::border_style(focused));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match state {
        SendState::Done(response) => {
            let fallback;
            let text = match prepared {
                Some(text) => text,
                None => {
                    fallback = build_response_text(response);
                    &fallback
                }
            };

            let [tabs, content] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);
            // The tabs sit on the left of the row, the status summary on the
            // right — the Length reserves exactly its width, so both fit on
            // the narrowest terminal.
            let status_width = text.status_line.width() as u16;
            let [left, right] =
                Layout::horizontal([Constraint::Min(12), Constraint::Length(status_width)])
                    .areas(tabs);
            frame.render_widget(
                Tabs::new(["Body", "Headers"])
                    .select(tab.index())
                    .style(view::title())
                    .highlight_style(view::accent()),
                left,
            );
            frame.render_widget(
                Paragraph::new(text.status_line.clone()).right_aligned(),
                right,
            );

            let section = match tab {
                ResponseTab::Body => &text.body,
                ResponseTab::Headers => &text.headers,
            };
            let total = section.len();
            let offset = scroll.min(max_offset(total, content.height));

            let window: Vec<&str> = section
                .iter()
                .skip(offset as usize)
                .take(content.height as usize)
                .map(String::as_str)
                .collect();

            frame.render_widget(Paragraph::new(window.join("\n")), content);
            render_scrollbar(frame, area, total, offset, content.height);
        }
        state => {
            let (message, style) = match state {
                SendState::Idle => ("No response yet", view::dim()),
                SendState::Sending => ("Sending…", view::warning()),
                SendState::Failed(error) => (error.as_str(), view::error()),
                SendState::Done(_) => unreachable!("handled above"),
            };

            frame.render_widget(Paragraph::new(message).style(style), inner);
        }
    }
}

fn render_scrollbar(frame: &mut Frame, area: Rect, total: usize, offset: u16, viewport: u16) {
    // The thumb rides the right border and only shows up once the content
    // actually overflows. `ScrollbarState` positions run in [0,
    // content_length - 1], so feeding it one "slot" per reachable offset
    // makes its maximum coincide with our clamp: the thumb then reaches the
    // exact bottom of the track when the last line is on screen.
    if total > viewport as usize {
        let slots = total - viewport as usize + 1;
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
