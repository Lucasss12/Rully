use crate::style;
use reqwest::StatusCode;
use reqwest::header::{CONTENT_LENGTH, HOST, HeaderMap, HeaderValue};
use rully_core::http;
use rully_core::request::Request;
use rully_core::response::Response;

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

fn status_color_code(status: &StatusCode) -> &'static str {
    match status.as_u16() {
        200..=299 => "32",
        300..=399 => "33",
        _ => "31",
    }
}

fn header_lines(headers: &HeaderMap, arrow: &str, paint: fn(&str) -> String) -> Vec<String> {
    headers
        .iter()
        .map(|(name, value)| {
            let value = value.to_str().unwrap_or("[non-UTF8]");

            format!(
                "{arrow} Header: {}: {}",
                paint(name.as_ref()),
                display_header_value(name.as_str(), value)
            )
        })
        .collect()
}

fn request_line(request: &Request) -> String {
    let target = match request.url.query() {
        Some(query) => format!("{}?{query}", request.url.path()),
        None => request.url.path().to_string(),
    };

    format!("{} {target} HTTP/1.1", request.method)
}

fn request_header_lines(headers: &HeaderMap) -> Vec<String> {
    header_lines(headers, "→", style::yellow)
}

fn added_header_lines(request: &Request) -> Vec<String> {
    let mut added = http::default_headers();

    if let Ok(host) = HeaderValue::from_str(&request.host_header()) {
        added.insert(HOST, host);
    }

    if let Some(length) = request.content_length_header()
        && let Ok(value) = HeaderValue::from_str(&length.to_string())
    {
        added.insert(CONTENT_LENGTH, value);
    }

    let mut only_added = HeaderMap::new();

    for (name, value) in &added {
        if !request.headers.contains_key(name) {
            only_added.append(name.clone(), value.clone());
        }
    }

    header_lines(&only_added, "→", style::cyan)
}

fn response_header_lines(headers: &HeaderMap) -> Vec<String> {
    header_lines(headers, "←", style::cyan)
}

fn style_status(status: &StatusCode) -> String {
    style::paint(status_color_code(status), &status.to_string())
}

fn display_summary(response: &Response) {
    match response.content_type() {
        Some(content_type) => println!(
            "← {} · {:.2}ms · {} · {}\n",
            style_status(&response.status),
            response.duration_ms(),
            format_size(response.size),
            style::cyan(content_type)
        ),
        None => println!(
            "← {} · {:.2}ms · {}\n",
            style_status(&response.status),
            response.duration_ms(),
            format_size(response.size)
        ),
    }
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

fn display_request_line(request: &Request) {
    println!("{}\n", request_line(request));
}

fn display_request_headers(request: &Request) {
    for line in request_header_lines(&request.headers)
        .into_iter()
        .chain(added_header_lines(request))
    {
        println!("{line}");
    }
    println!("");
}

fn display_response_headers(headers: &HeaderMap) {
    for line in response_header_lines(headers) {
        println!("{line}");
    }
    println!("");
}

pub(crate) fn display_response(response: &Response, request: &Request, verbose: bool) {
    if verbose {
        display_request_line(request);
        display_request_headers(request);
    }

    display_summary(response);

    if verbose {
        display_response_headers(&response.headers);
    }

    if !response.body.is_empty() {
        display_body(&response.body, response.content_type());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{AUTHORIZATION, HeaderValue};
    use rully_core::request::Request;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();

        for (name, value) in pairs {
            headers.append(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }

        headers
    }

    #[test]
    fn formats_sizes_below_one_kilobyte() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn formats_sizes_from_one_kilobyte() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn pretty_prints_a_json_body() {
        let pretty = pretty_print_json(r#"{"name":"John","id":1}"#).unwrap();

        assert!(pretty.contains('\n'), "expected an indented body: {pretty}");
        assert!(pretty.contains(r#""name": "John""#), "{pretty}");
    }

    #[test]
    fn refuses_to_pretty_print_a_body_that_is_not_json() {
        assert_eq!(pretty_print_json("hello"), None);
        assert_eq!(pretty_print_json(""), None);
        assert_eq!(pretty_print_json("{invalid}"), None);
    }

    #[test]
    fn treats_credentials_as_sensitive() {
        for name in [
            "authorization",
            "cookie",
            "set-cookie",
            "proxy-authorization",
            "x-api-key",
        ] {
            assert!(is_sensitive_header(name), "{name} should be sensitive");
        }
    }

    #[test]
    fn matches_sensitive_headers_regardless_of_case() {
        assert!(is_sensitive_header("AUTHORIZATION"));
        assert!(is_sensitive_header("Authorization"));
    }

    #[test]
    fn keeps_regular_headers_visible() {
        for name in ["accept", "content-type", "user-agent", "x-request-id"] {
            assert!(!is_sensitive_header(name), "{name} should not be sensitive");
        }
    }

    #[test]
    fn redacts_the_value_of_sensitive_headers() {
        assert_eq!(
            display_header_value("authorization", "Bearer token"),
            "[redacted]"
        );
        assert_eq!(
            display_header_value("accept", "application/json"),
            "application/json"
        );
    }

    #[test]
    fn lists_request_headers_with_their_values() {
        let lines = request_header_lines(&headers(&[("accept", "application/json")]));

        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("→ Header: "), "{}", lines[0]);
        assert!(
            lines[0].contains("accept: application/json"),
            "{}",
            lines[0]
        );
    }

    #[test]
    fn redacts_sensitive_request_headers() {
        let lines = request_header_lines(&headers(&[("authorization", "Bearer secret")]));

        assert!(lines[0].contains("[redacted]"), "{}", lines[0]);
        assert!(!lines[0].contains("secret"), "{}", lines[0]);
    }

    #[test]
    fn lists_every_response_header_including_duplicates() {
        let lines = response_header_lines(&headers(&[
            ("content-type", "application/json"),
            ("accept", "text/plain"),
            ("accept", "application/json"),
        ]));

        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| line.starts_with("← Header: ")));
    }

    #[test]
    fn redacts_sensitive_response_headers() {
        let lines = response_header_lines(&headers(&[("set-cookie", "session=abc")]));

        assert!(lines[0].contains("[redacted]"), "{}", lines[0]);
        assert!(!lines[0].contains("session=abc"), "{}", lines[0]);
    }

    #[test]
    fn redacts_for_display_without_erasing_the_request_headers() {
        let mut request = Request::new(reqwest::Method::GET, "https://example.com").unwrap();
        request.headers = headers(&[("authorization", "Bearer secret")]);

        let lines = request_header_lines(&request.headers);

        assert!(!lines[0].contains("secret"), "{}", lines[0]);
        assert_eq!(request.headers[AUTHORIZATION], "Bearer secret");
    }

    #[test]
    fn colors_success_redirect_and_error_statuses() {
        assert_eq!(status_color_code(&StatusCode::OK), "32");
        assert_eq!(status_color_code(&StatusCode::CREATED), "32");
        assert_eq!(status_color_code(&StatusCode::MOVED_PERMANENTLY), "33");
        assert_eq!(status_color_code(&StatusCode::FOUND), "33");
        assert_eq!(status_color_code(&StatusCode::NOT_FOUND), "31");
        assert_eq!(status_color_code(&StatusCode::INTERNAL_SERVER_ERROR), "31");
    }

    #[test]
    fn always_shows_the_status_code_whatever_the_terminal() {
        for status in [StatusCode::OK, StatusCode::FOUND, StatusCode::NOT_FOUND] {
            let rendered = style_status(&status);

            assert!(
                rendered.contains(&status.to_string()),
                "{status} should render its code, got {rendered}"
            );
        }
    }

    #[test]
    fn reports_the_headers_rully_adds_on_top_of_the_request() {
        let request = Request::new(reqwest::Method::GET, "https://api.example.com/users").unwrap();

        let lines = added_header_lines(&request);

        assert_eq!(lines.len(), 4, "{lines:?}");
        assert!(
            lines.iter().any(|line| line.contains("accept: */*")),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("user-agent: rully/")),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("host: api.example.com")),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("accept-encoding: gzip")),
            "{lines:?}"
        );
    }

    #[test]
    fn adds_a_content_length_only_when_the_request_has_a_body() {
        let without_body = Request::new(reqwest::Method::GET, "https://example.com").unwrap();
        assert!(
            !added_header_lines(&without_body)
                .iter()
                .any(|l| l.contains("content-length"))
        );

        let mut with_body = Request::new(reqwest::Method::POST, "https://example.com").unwrap();
        with_body.body = Some("héllo".to_string());

        let lines = added_header_lines(&with_body);
        let length = lines
            .iter()
            .find(|line| line.contains("content-length"))
            .unwrap();

        assert!(length.contains(": 6"), "{length}");
    }

    #[test]
    fn never_repeats_a_header_the_caller_already_set() {
        let mut request = Request::new(reqwest::Method::GET, "https://example.com").unwrap();
        request.headers = headers(&[
            ("accept", "application/json"),
            ("accept-encoding", "identity"),
            ("user-agent", "custom/1.0"),
            ("host", "elsewhere.example.com"),
        ]);

        let own = request_header_lines(&request.headers);
        let added = added_header_lines(&request);

        assert_eq!(own.len(), 4);
        assert!(added.is_empty(), "nothing left to add, got {added:?}");
    }

    #[test]
    fn renders_the_request_line_with_the_method_and_path() {
        let request = Request::new(reqwest::Method::POST, "https://example.com/users").unwrap();

        assert_eq!(request_line(&request), "POST /users HTTP/1.1");
    }

    #[test]
    fn keeps_the_query_in_the_request_line() {
        let request = Request::new(
            reqwest::Method::GET,
            "https://example.com/search?q=rust&page=2",
        )
        .unwrap();

        assert_eq!(request_line(&request), "GET /search?q=rust&page=2 HTTP/1.1");
    }

    #[test]
    fn uses_a_root_path_when_the_url_has_none() {
        let request = Request::new(reqwest::Method::GET, "https://example.com").unwrap();

        assert_eq!(request_line(&request), "GET / HTTP/1.1");
    }

    #[test]
    fn leaves_the_fragment_out_of_the_request_line() {
        let request = Request::new(reqwest::Method::GET, "https://example.com/users#top").unwrap();

        assert_eq!(request_line(&request), "GET /users HTTP/1.1");
    }

    #[test]
    fn marks_added_headers_so_they_can_be_told_apart_from_own_ones() {
        let request = Request::new(reqwest::Method::GET, "https://example.com").unwrap();

        for line in added_header_lines(&request) {
            assert!(line.starts_with("→ Header: "), "{line}");
        }
    }
}
