use super::{EditMode, Focus};
use crossterm::event::{KeyCode, KeyModifiers};

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

    EditUrl,
    EditCurrent,
    SubmitEdit,
    CancelEdit,
}

pub(super) struct Binding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
    help: &'static str,
    pub action: Action,
}

const ROOT_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        help: "↑ up",
        action: Action::SelectPrevious,
    },
    Binding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        help: "↓ down",
        action: Action::SelectNext,
    },
    Binding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        help: "↵ open",
        action: Action::Activate,
    },
    Binding {
        key: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        help: "tab",
        action: Action::NextFocus,
    },
];

const COLLECTION_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        help: "↑ up",
        action: Action::SelectPrevious,
    },
    Binding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        help: "↓ down",
        action: Action::SelectNext,
    },
    Binding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        help: "↵ open",
        action: Action::Activate,
    },
    Binding {
        key: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        help: "esc back",
        action: Action::GoBack,
    },
];

const MENU_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        help: "↑ up",
        action: Action::MenuPrevious,
    },
    Binding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        help: "↓ down",
        action: Action::MenuNext,
    },
    Binding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        help: "↵ open",
        action: Action::MenuConfirm,
    },
    Binding {
        key: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        help: "esc back",
        action: Action::MenuCancel,
    },
];

const REQUEST_BAR_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Char('m'),
        modifiers: KeyModifiers::NONE,
        help: "m method",
        action: Action::OpenMethodMenu,
    },
    Binding {
        key: KeyCode::Char('e'),
        modifiers: KeyModifiers::NONE,
        help: "e edit url",
        action: Action::EditUrl,
    },
    Binding {
        key: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        help: "esc back",
        action: Action::FocusLeftPanel,
    },
];

const REQUEST_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Left,
        modifiers: KeyModifiers::NONE,
        help: "← prev",
        action: Action::TabPrevious,
    },
    Binding {
        key: KeyCode::Right,
        modifiers: KeyModifiers::NONE,
        help: "→ next",
        action: Action::TabNext,
    },
    Binding {
        key: KeyCode::Char('e'),
        modifiers: KeyModifiers::NONE,
        help: "e edit",
        action: Action::EditCurrent,
    },
    Binding {
        key: KeyCode::Char('b'),
        modifiers: KeyModifiers::NONE,
        help: "b body",
        action: Action::OpenBodyMenu,
    },
    Binding {
        key: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        help: "a auth",
        action: Action::OpenAuthMenu,
    },
    Binding {
        key: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        help: "esc back",
        action: Action::FocusLeftPanel,
    },
];

const RESPONSE_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        help: "q quit",
        action: Action::Quit,
    },
    Binding {
        key: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        help: "tab",
        action: Action::NextFocus,
    },
    Binding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        help: "esc back",
        action: Action::FocusLeftPanel,
    },
];

const EDIT_KEYS: &[Binding] = &[
    Binding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        help: "esc cancel",
        action: Action::CancelEdit,
    },
    Binding {
        key: KeyCode::Char('s'),
        modifiers: KeyModifiers::CONTROL,
        help: "ctrl+s save",
        action: Action::SubmitEdit,
    },
];

pub(super) fn current_binding(
    focus: Focus,
    open: Option<&str>,
    menu_open: bool,
    edit_mode: EditMode,
) -> &'static [Binding] {
    if edit_mode != EditMode::None {
        return EDIT_KEYS;
    }
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
