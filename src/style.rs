use std::io::IsTerminal;

pub fn paint(code: &str, text: &str) -> String {
    if std::io::stdout().is_terminal() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn yellow(text: &str) -> String {
    paint("33", text)
}

pub fn cyan(text: &str) -> String {
    paint("36", text)
}
