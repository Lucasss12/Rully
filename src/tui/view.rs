use ratatui::style::{Color, Modifier, Style};

/// The app accent: focused borders, the active environment dot, the app name.
pub(super) fn accent() -> Style {
    Style::new().fg(Color::Cyan)
}

/// HTTP status classes: success green, redirect yellow, error red.
pub(super) fn status_style(code: u16) -> Style {
    match code {
        200..=299 => Style::new().fg(Color::Green),
        300..=399 => Style::new().fg(Color::Yellow),
        _ => Style::new().fg(Color::Red),
    }
}

/// HTTP verbs, same semantics as the status classes.
pub(super) fn method_style(method: &str) -> Style {
    match method {
        "GET" => Style::new().fg(Color::Green),
        "POST" => Style::new().fg(Color::Yellow),
        "PUT" | "PATCH" => Style::new().fg(Color::Blue),
        "DELETE" => Style::new().fg(Color::Red),
        _ => Style::new(),
    }
}

pub(super) fn dim() -> Style {
    Style::new().add_modifier(Modifier::DIM)
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

pub(super) fn border_style(focused: bool) -> Style {
    if focused { accent() } else { Style::new() }
}
