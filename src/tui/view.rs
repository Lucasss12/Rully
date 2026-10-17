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
