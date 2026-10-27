use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Paragraph, Tabs, Wrap},
};
use ratatui_textarea::TextArea;

use super::{Draft, EditMode, content, view};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum RequestTab {
    Headers,
    Body,
    Auth,
}

impl RequestTab {
    pub const ALL: [Self; 3] = [Self::Headers, Self::Body, Self::Auth];

    fn index(self) -> usize {
        match self {
            RequestTab::Headers => 0,
            RequestTab::Body => 1,
            RequestTab::Auth => 2,
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

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    draft: Option<&Draft>,
    tab: RequestTab,
    focused: bool,
    edit_mode: EditMode,
    edit_input: &TextArea<'_>,
) {
    let title = match draft {
        Some(draft) => {
            let auth = content::AuthKind::of(&draft.headers);
            if auth == content::AuthKind::None {
                format!("Request · {}", draft.body_kind.label())
            } else {
                format!("Request · {} · {}", draft.body_kind.label(), auth.label())
            }
        }
        None => "Request".to_string(),
    };
    let block = Block::bordered()
        .title(title)
        .border_style(view::border_style(focused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [tabs, content] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

    frame.render_widget(
        Tabs::new(["Headers", "Body", "Auth"]).select(tab.index()),
        tabs,
    );

    let editing = match tab {
        RequestTab::Headers => edit_mode == EditMode::Headers,
        RequestTab::Body => edit_mode == EditMode::Body,
        RequestTab::Auth => edit_mode == EditMode::Auth,
    };

    if editing {
        frame.render_widget(edit_input, content);
    } else {
        frame.render_widget(paragraph(draft, tab), content);
    }
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
        RequestTab::Auth => {
            let lines = content::auth_lines(&draft.headers);
            if lines.is_empty() {
                return Paragraph::new("No auth").style(view::dim());
            }
            Paragraph::new(lines).wrap(Wrap { trim: false })
        }
    }
}
