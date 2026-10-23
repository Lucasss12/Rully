use super::Focus;
use crossterm::event::KeyCode;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Quit,
    SelectPrevious,
    SelectNext,
    Activate,
    GoBack,

    NextFocus,
    MenuPrevious,
    MenuNext,
    MenuConfirm,
    MenuCancel,

    TabPrevious,
    TabNext,
    OpenMethodMenu,
    OpenBodyMenu,
    OpenAuthMenu,
    FocusLeftPanel,
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
    Binding {
        key: KeyCode::Tab,
        help: "tab",
        action: Action::NextFocus,
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
        key: KeyCode::Enter,
        help: "↵ open",
        action: Action::Activate,
    },
    Binding {
        key: KeyCode::Tab,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        help: "esc back",
        action: Action::GoBack,
    },
];

const MENU_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Up,
        help: "↑ up",
        action: Action::MenuPrevious,
    },
    Binding {
        key: KeyCode::Down,
        help: "↓ down",
        action: Action::MenuNext,
    },
    Binding {
        key: KeyCode::Enter,
        help: "↵ open",
        action: Action::MenuConfirm,
    },
    Binding {
        key: KeyCode::Tab,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        help: "esc back",
        action: Action::MenuCancel,
    },
];

const REQUEST_BAR_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Char('m'),
        help: "m method",
        action: Action::OpenMethodMenu,
    },
    Binding {
        key: KeyCode::Tab,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        help: "esc back",
        action: Action::FocusLeftPanel,
    },
];

const REQUEST_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Left,
        help: "← prev",
        action: Action::TabPrevious,
    },
    Binding {
        key: KeyCode::Right,
        help: "→ next",
        action: Action::TabNext,
    },
    Binding {
        key: KeyCode::Char('b'),
        help: "b body",
        action: Action::OpenBodyMenu,
    },
    Binding {
        key: KeyCode::Char('a'),
        help: "a auth",
        action: Action::OpenAuthMenu,
    },
    Binding {
        key: KeyCode::Tab,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        help: "esc back",
        action: Action::FocusLeftPanel,
    },
];

const RESPONSE_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Tab,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        help: "esc back",
        action: Action::FocusLeftPanel,
    },
];

pub(super) fn current_binding(
    focus: Focus,
    open: Option<&str>,
    menu_open: bool,
) -> &'static [Binding] {
    if menu_open {
        return MENU_KEYS;
    }

    match focus {
        Focus::LeftPanel => match open {
            None => ROOT_KEYS,
            Some(_) => COLLECTION_KEYS,
        },
        Focus::RequestBar => REQUEST_BAR_KEYS,
        Focus::Request => REQUEST_KEYS,
        Focus::Response => RESPONSE_KEYS,
    }
}

pub(super) fn help_line(bindings: &[Binding]) -> String {
    bindings
        .iter()
        .map(|b| b.help)
        .collect::<Vec<_>>()
        .join("  ·  ")
}
