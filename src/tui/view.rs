use ratatui::style::{Color, Style};

/// The Tokyo Night palette the interface is built on. All colors are RGB so
/// the theme renders identically on any truecolor terminal.
const ACCENT: Color = Color::Rgb(122, 162, 247); // #7AA2F7 blue
const TITLE: Color = Color::Rgb(187, 154, 247); // #BB9AF7 violet
const BORDER_IDLE: Color = Color::Rgb(86, 95, 137); // #565F89 grey violet
const TEXT_SOFT: Color = Color::Rgb(169, 177, 214); // #A9B1D6 soft blue grey
const OK: Color = Color::Rgb(158, 206, 106); // #9ECE6A green
const WARN: Color = Color::Rgb(224, 175, 104); // #E0AF68 yellow
const ERROR: Color = Color::Rgb(247, 118, 142); // #F7768E red

/// Interactive elements: focused borders, keys, methods, selected tab.
pub(super) fn accent() -> Style {
    Style::new().fg(ACCENT)
}

/// The key glyph in a help entry (`s send`), same accent as the borders.
pub(super) fn key() -> Style {
    Style::new().fg(ACCENT)
}

/// Panel titles and tab labels, violet to separate them from the content.
pub(super) fn title() -> Style {
    Style::new().fg(TITLE)
}

/// Idle borders: present but receded, so the focused panel keeps the accent.
pub(super) fn border_style(focused: bool) -> Style {
    if focused {
        key()
    } else {
        Style::new().fg(BORDER_IDLE)
    }
}

/// HTTP status classes: success green, redirect yellow, error red.
pub(super) fn status_style(code: u16) -> Style {
    match code {
        200..=299 => Style::new().fg(OK),
        300..=399 => Style::new().fg(WARN),
        _ => Style::new().fg(ERROR),
    }
}

/// Secondary text: placeholders, help actions, separators.
pub(super) fn dim() -> Style {
    Style::new().fg(TEXT_SOFT)
}

/// An in-flight state rather than an absence: pairs with `x cancel`.
pub(super) fn warning() -> Style {
    Style::new().fg(WARN)
}

/// Errors, wherever they are reported.
pub(super) fn error() -> Style {
    Style::new().fg(ERROR)
}

pub(super) fn bread_crumb(open: Option<&str>) -> String {
    match open {
        None => "Collections".to_string(),
        Some(name) => format!("Collections/{name}"),
    }
}

pub(super) fn empty_message(open: Option<&str>) -> &'static str {
    match open {
        None => "No collections found",
        Some(_) => "No requests found",
    }
}
