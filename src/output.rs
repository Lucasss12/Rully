use crate::cli::Cli;
use reqwest::header::CONTENT_TYPE;

fn content_type(headers: &reqwest::header::HeaderMap) -> Option<&str> {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or("").trim())
        .filter(|value| !value.is_empty())
}

fn pretty_print_json(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    serde_json::to_string_pretty(&value).ok()
}

fn format_size(bytes: usize) -> String {
    let units = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", units[unit])
    } else {
        format!("{size:.1} {}", units[unit])
    }
}

fn is_sensitive_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization")
        || name.eq_ignore_ascii_case("cookie")
        || name.eq_ignore_ascii_case("set-cookie")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("x-api-key")
}

fn display_header_value(name: &str, value: &str) -> String {
    if is_sensitive_header(name) {
        "[redacted]".to_string()
    } else {
        value.to_string()
    }
}

fn display_request_headers(cli: &Cli) {
    for header in &cli.headers {
        let Some((name, value)) = header.split_once(':') else {
            continue;
        };

        let name = name.trim();
        let value = value.trim();
        println!("→ Header: {}: {}", name, display_header_value(name, value));
    }
}

fn display_response_headers(headers: &reqwest::header::HeaderMap) {
    for (name, value) in headers {
        let value = value.to_str().unwrap_or("[non-UTF8]");
        println!(
            "← Header: {}: {}",
            name,
            display_header_value(name.as_str(), value)
        );
    }
}

fn display_summary(
    status: &reqwest::StatusCode,
    body: &str,
    duration_ms: f64,
    content_type: Option<&str>,
) {
    println!("---");
    match content_type {
        Some(ct) => println!(
            "← {} · {:.2}ms · {} · {ct}",
            status,
            duration_ms,
            format_size(body.len())
        ),
        None => println!(
            "← {} · {:.2}ms · {}",
            status,
            duration_ms,
            format_size(body.len())
        ),
    }
    println!("---");
}

fn display_body(body: &str, content_type: Option<&str>) {
    println!("Body:");
    if content_type == Some("application/json")
        && let Some(pretty) = pretty_print_json(body)
    {
        println!("{pretty}");
        return;
    }
    println!("{body}");
}

pub(crate) fn display_response(
    status: &reqwest::StatusCode,
    response_headers: &reqwest::header::HeaderMap,
    body: &str,
    cli: &Cli,
    duration_ms: f64,
) {
    if cli.verbose {
        display_request_headers(cli);
    }

    let content_type = content_type(response_headers);

    display_summary(status, body, duration_ms, content_type);

    if cli.verbose {
        display_response_headers(response_headers);
    }

    if !body.is_empty() {
        display_body(body, content_type);
    }
}
