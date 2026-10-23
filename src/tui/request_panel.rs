use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Paragraph, Tabs, Wrap},
};

use super::{Draft, content, view};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum RequestTab {
    Headers,
    Body,
}

impl RequestTab {
    fn index(self) -> usize {
        match self {
            RequestTab::Headers => 0,
            RequestTab::Body => 1,
        }
    }

    pub(super) fn switch_tabs(self) -> Self {
        match self {
            RequestTab::Headers => RequestTab::Body,
            RequestTab::Body => RequestTab::Headers,
        }
    }
}

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    draft: Option<&Draft>,
    tab: RequestTab,
    focused: bool,
) {
    let title = match draft {
        Some(draft) => format!("Request · {}", draft.body_kind.label()),
        None => "Request".to_string(),
    };
    let block = Block::bordered()
        .title(title)
        .border_style(view::border_style(focused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [tabs, content] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

    frame.render_widget(Tabs::new(["Headers", "Body"]).select(tab.index()), tabs);
    frame.render_widget(paragraph(draft, tab), content);
}

fn paragraph(draft: Option<&Draft>, tab: RequestTab) -> Paragraph<'_> {
    let Some(draft) = draft else {
        return Paragraph::new("No request selected").style(view::dim());
    };

    match tab {
        RequestTab::Headers => {
            if draft.headers.is_empty() {
                return Paragraph::new("No headers").style(view::dim());
            }
            Paragraph::new(content::header_lines(&draft.headers)).wrap(Wrap { trim: false })
        }
        RequestTab::Body if draft.body_kind == content::BodyKind::None => {
            Paragraph::new("No body").style(view::dim())
        }
        RequestTab::Body => match draft.body.as_deref() {
            None => Paragraph::new("No body").style(view::dim()),
            Some(body) => {
                Paragraph::new(content::body_text(body, draft.body_kind)).wrap(Wrap { trim: false })
            }
        },
    }
}
