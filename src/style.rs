use std::io::IsTerminal;

fn paint(code: &str, text: &str) -> String {
    if std::io::stdout().is_terminal() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn green(text: &str) -> String {
    paint("32", text)
}

pub fn yellow(text: &str) -> String {
    paint("33", text)
}

pub fn red(text: &str) -> String {
    paint("31", text)
}

pub fn cyan(text: &str) -> String {
    paint("36", text)
}
