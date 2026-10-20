use ratatui::style::{Color, Modifier, Style};

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
    if focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new()
    }
}
