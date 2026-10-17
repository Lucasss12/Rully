use crossterm::event::KeyCode;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Quit,
    SelectPrevious,
    SelectNext,
    Activate,
    GoBack,
}

pub(super) struct Binding {
    pub key: KeyCode,
    help: &'static str,
    pub action: Action,
}

const ROOT_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Up,
        help: "↑ up",
        action: Action::SelectPrevious,
    },
    Binding {
        key: KeyCode::Down,
        help: "↓ down",
        action: Action::SelectNext,
    },
    Binding {
        key: KeyCode::Enter,
        help: "↵ open",
        action: Action::Activate,
    },
];

const COLLECTION_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Up,
        help: "↑ up",
        action: Action::SelectPrevious,
    },
    Binding {
        key: KeyCode::Down,
        help: "↓ down",
        action: Action::SelectNext,
    },
    Binding {
        key: KeyCode::Esc,
        help: "esc back",
        action: Action::GoBack,
    },
];

pub(super) fn current_binding(open: Option<&str>) -> &'static [Binding] {
    match open {
        None => ROOT_KEYS,
        Some(_) => COLLECTION_KEYS,
    }
}

pub(super) fn help_line(bindings: &[Binding]) -> String {
    bindings
        .iter()
        .map(|b| b.help)
        .collect::<Vec<_>>()
        .join("  ·  ")
}
