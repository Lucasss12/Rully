use crate::request::Request;
use crate::response::Response;
use crate::style;
use reqwest::StatusCode;
use reqwest::header::HeaderMap;

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

fn display_request_headers(request: &Request) {
    for (name, value) in &request.headers {
        let value = value.to_str().unwrap_or("[non-UTF8]");

        println!(
            "→ Header: {}: {}",
            style::yellow(name.as_ref()),
            display_header_value(name.as_str(), value)
        );
    }
}

fn display_response_headers(headers: &HeaderMap) {
    for (name, value) in headers {
        let value = value.to_str().unwrap_or("[non-UTF8]");
        println!(
            "← Header: {}: {}",
            style::cyan(name.as_ref()),
            display_header_value(name.as_str(), value)
        );
    }
}

fn style_status(status: &StatusCode) -> String {
    let code = status.to_string();
    match status.as_u16() {
        200..=299 => style::green(&code),
        300..=399 => style::yellow(&code),
        _ => style::red(&code),
    }
}

fn display_summary(response: &Response) {
    println!("---");

    match response.content_type() {
        Some(content_type) => println!(
            "← {} · {:.2}ms · {} · {}",
            style_status(&response.status),
            response.duration_ms(),
            format_size(response.size),
            style::cyan(content_type)
        ),
        None => println!(
            "← {} · {:.2}ms · {}",
            style_status(&response.status),
            response.duration_ms(),
            format_size(response.size)
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

pub(crate) fn display_response(response: &Response, request: &Request, verbose: bool) {
    if verbose {
        display_request_headers(request);
    }

    let content_type = response.content_type();

    display_summary(response);

    if verbose {
        display_response_headers(&response.headers);
    }

    if !response.body.is_empty() {
        display_body(&response.body, content_type);
    }
}
