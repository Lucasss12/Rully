use crate::errors::AppError;
use crate::request::Request;
use reqwest::Method;
use reqwest::header::{HeaderName, HeaderValue};
use std::fs;

pub struct HttpFile {
    method: String,
    url: String,
    body: Option<String>,
    headers: Vec<(String, String)>,
}

pub fn read(path: &str) -> Result<HttpFile, AppError> {
    let source = fs::read_to_string(path).map_err(AppError::FileRead)?;
    parse(&source)
}

pub fn parse(source: &str) -> Result<HttpFile, AppError> {
    let lines: Vec<&str> = source.lines().collect();

    let mut index = 0;
    while index < lines.len() && is_skippable(lines[index]) {
        index += 1;
    }

    let request_line = lines
        .get(index)
        .ok_or_else(|| AppError::InvalidHttpFile("empty request file".to_string()))?;
    let (method, url) = request_line
        .trim()
        .split_once(' ')
        .ok_or_else(|| AppError::InvalidHttpFile("invalid request line".to_string()))?;

    index += 1;

    let mut headers = Vec::new();
    while index < lines.len() {
        let line = lines[index].trim();

        if line.is_empty() {
            break;
        }

        if line.starts_with('#') {
            index += 1;
            continue;
        }

        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| AppError::InvalidHttpFile(format!("invalid header line: {line}")))?;

        headers.push((name.trim().to_string(), value.trim().to_string()));
        index += 1;
    }

    index += 1;

    let body = if index >= lines.len() {
        None
    } else {
        Some(lines[index..].join("\n"))
    };

    Ok(HttpFile {
        method: method.to_string(),
        url: url.to_string(),
        headers,
        body,
    })
}

/// The skeleton written by `rully new`. It teaches the format on first use.
const TEMPLATE: &str = "# <METHOD> <URL>\n\
                        # One header per line, as \"Name: value\".\n\
                        # Everything after the first blank line is the request body.\n\
                        \n\
                        GET https://api.example.com\n";

pub fn template() -> &'static str {
    TEMPLATE
}

pub fn is_request_path(path: &str) -> bool {
    path.ends_with(".http")
}

/// Serialises a request back to the file format.
///
/// This is the inverse of `parse`, so `parse(&to_string(r)?) == r`. Comments
/// are not part of a `Request`, so a serialised file never carries any: the
/// format only ever adds them to a file the user wrote by hand.
pub fn to_string(request: &Request) -> Result<String, AppError> {
    let mut out = format!("{} {}\n", request.method, request.url);

    for (name, value) in request.headers.iter() {
        let value = value
            .to_str()
            .map_err(|_| AppError::InvalidHeader(format!("non-UTF8 value: {name}")))?;

        out.push_str(&format!("{name}: {value}\n"));
    }

    if let Some(body) = &request.body {
        out.push('\n');
        out.push_str(body);
        // Exactly one newline, always. `parse` drops the last line break, so a
        // body ending in "\n" needs one extra to survive the round trip.
        out.push('\n');
    }

    Ok(out)
}

fn is_skippable(line: &str) -> bool {
    let line = line.trim();
    line.is_empty() || line.starts_with('#')
}

fn to_method(method: &str) -> Result<Method, AppError> {
    Method::from_bytes(method.as_bytes())
        .map_err(|_| AppError::InvalidHttpFile(format!("unsupported method: {method}")))
}

fn add_headers(request: &mut Request, headers: &[(String, String)]) -> Result<(), AppError> {
    for (name, value) in headers {
        let header_name = HeaderName::from_bytes(name.trim().as_bytes()).map_err(|_| {
            AppError::InvalidHeader(format!("Invalid header name: {}", name.trim()))
        })?;

        let header_value = HeaderValue::from_str(value.trim())
            .map_err(|_| AppError::InvalidHeader(format!("Invalid header value: {name}")))?;

        request.headers.append(header_name, header_value);
    }

    Ok(())
}

impl TryFrom<HttpFile> for Request {
    type Error = AppError;

    fn try_from(file: HttpFile) -> Result<Self, AppError> {
        let method = to_method(&file.method)?;
        let mut request = Request::new(method, &file.url)?;

        add_headers(&mut request, &file.headers)?;
        request.body = file.body;

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_file() {
        let path = std::env::temp_dir().join("rully-reads-a-file.http");
        fs::write(&path, "GET https://api.example.com").unwrap();
        let file = super::read(path.to_str().unwrap()).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(file.method, "GET");
        assert_eq!(file.url, "https://api.example.com");
    }

    #[test]
    fn parses_a_get_request() {
        let file = parse("GET https://api.example.com").unwrap();

        assert_eq!(file.method, "GET");
        assert_eq!(file.url, "https://api.example.com");
    }

    #[test]
    fn ignores_comments() {
        let file = parse("# a note\nGET https://api.example.com").unwrap();

        assert_eq!(file.method, "GET");
    }

    #[test]
    fn fails_on_an_empty_file() {
        let result = parse("");

        assert!(matches!(result, Err(AppError::InvalidHttpFile(_))));
    }

    #[test]
    fn fails_on_a_missing_request_line() {
        let result = parse("# only a comment");

        assert!(matches!(result, Err(AppError::InvalidHttpFile(_))));
    }

    #[test]
    fn fails_on_a_request_line_without_url() {
        let result = parse("GET");

        assert!(matches!(result, Err(AppError::InvalidHttpFile(_))));
    }

    #[test]
    fn reports_a_missing_file() {
        let result = super::read("missing.http");

        assert!(matches!(result, Err(AppError::FileRead(_))));
    }

    #[test]
    fn has_no_headers_without_any() {
        let file = parse("GET https://api.example.com").unwrap();

        assert!(file.headers.is_empty());
    }

    #[test]
    fn parses_headers() {
        let file = parse(
            "GET https://api.example.com\n\
             Accept: application/json\n\
             Authorization: Bearer abc123",
        )
        .unwrap();

        assert_eq!(file.headers.len(), 2);
        assert_eq!(file.headers[0].0, "Accept");
        assert_eq!(file.headers[0].1, "application/json");
        assert_eq!(file.headers[1].0, "Authorization");
        assert_eq!(file.headers[1].1, "Bearer abc123");
    }

    #[test]
    fn keeps_duplicate_header_names() {
        let file = parse(
            "GET https://api.example.com\n\
             Accept: application/json\n\
             Accept: text/plain",
        )
        .unwrap();

        assert_eq!(file.headers.len(), 2);
    }

    #[test]
    fn parses_a_body() {
        let file = parse("GET https://api.example.com\n\n{\"name\": \"John\"}").unwrap();

        assert_eq!(file.body, Some("{\"name\": \"John\"}".to_string()));
    }

    #[test]
    fn has_no_body_when_nothing_follows_the_blank_line() {
        let file = parse("GET https://api.example.com\nAccept: application/json\n\n").unwrap();

        assert_eq!(file.body, None);
    }

    #[test]
    fn keeps_the_blank_lines_inside_a_body() {
        let file = parse("POST https://api.example.com\n\n{\n\n}").unwrap();

        assert_eq!(file.body, Some("{\n\n}".to_string()));
    }

    #[test]
    fn keeps_a_body_that_starts_with_a_blank_line() {
        let file = parse("POST https://api.example.com\n\n\nbody").unwrap();

        assert_eq!(file.body, Some("\nbody".to_string()));
    }

    #[test]
    fn ignores_comments_between_headers() {
        let file = parse(
            "GET https://api.example.com\n\
             Accept: application/json\n\
             # a note in the middle\n\
             Authorization: Bearer abc123",
        )
        .unwrap();

        assert_eq!(file.headers.len(), 2);
    }

    #[test]
    fn accepts_crlf_line_endings() {
        let file = parse("GET https://api.example.com\r\nAccept: application/json\r\n").unwrap();

        assert_eq!(file.method, "GET");
        assert_eq!(file.url, "https://api.example.com");
        assert_eq!(file.headers[0].1, "application/json");
    }

    #[test]
    fn fails_on_a_header_without_a_colon() {
        let result = parse("GET https://api.example.com\nBad Name");

        assert!(matches!(result, Err(AppError::InvalidHttpFile(_))));
    }

    fn to_request(source: &str) -> Result<Request, AppError> {
        parse(source).and_then(Request::try_from)
    }

    #[test]
    fn converts_method_and_url() {
        let request = to_request("POST https://api.example.com/users").unwrap();

        assert_eq!(request.method, Method::POST);
        assert_eq!(request.url.as_str(), "https://api.example.com/users");
    }

    #[test]
    fn converts_every_supported_method() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
            let request = to_request(&format!("{method} https://api.example.com")).unwrap();

            assert_eq!(request.method.as_str(), method);
        }
    }

    /// `Method::from_bytes` validates an RFC 7230 token rather than a fixed
    /// list, so a method is rejected only when it is not a token at all.
    /// `GE/T` is invalid because `/` is a separator, `GE!` is not, since `!`
    /// belongs to the token set.
    #[test]
    fn rejects_an_invalid_method() {
        let result = to_request("GE/T https://api.example.com");

        assert!(matches!(result, Err(AppError::InvalidHttpFile(_))));
    }

    #[test]
    fn accepts_any_valid_method() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
            let request = to_request(&format!("{method} https://api.example.com")).unwrap();

            assert_eq!(
                request.method,
                Method::from_bytes(method.as_bytes()).unwrap()
            );
        }
    }

    #[test]
    fn rejects_an_invalid_url() {
        let result = to_request("GET not-an-url");

        assert!(matches!(result, Err(AppError::InvalidUrl(_))));
    }

    #[test]
    fn converts_headers() {
        let request = to_request(
            "GET https://api.example.com\n\
             Accept: application/json\n\
             Authorization: Bearer abc123",
        )
        .unwrap();

        assert_eq!(request.headers["accept"], "application/json");
        assert_eq!(request.headers["authorization"], "Bearer abc123");
    }

    #[test]
    fn converts_duplicate_header_names() {
        let request = to_request(
            "GET https://api.example.com\n\
             Accept: application/json\n\
             Accept: text/plain",
        )
        .unwrap();

        let values: Vec<_> = request.headers.get_all("accept").iter().collect();

        assert_eq!(values.len(), 2);
    }

    #[test]
    fn rejects_an_invalid_header_name() {
        let result = to_request("GET https://api.example.com\nBad Name: value");

        assert!(matches!(result, Err(AppError::InvalidHeader(_))));
    }

    #[test]
    fn rejects_an_invalid_header_value() {
        let result = to_request("GET https://api.example.com\nAccept: a\rb");

        assert!(matches!(result, Err(AppError::InvalidHeader(_))));
    }

    #[test]
    fn converts_the_body() {
        let request = to_request("POST https://api.example.com\n\n{\"name\": \"John\"}").unwrap();

        assert_eq!(request.body.as_deref(), Some("{\"name\": \"John\"}"));
    }

    #[test]
    fn has_no_body_when_the_file_has_none() {
        let request = to_request("GET https://api.example.com").unwrap();

        assert!(request.body.is_none());
    }

    #[test]
    fn converts_a_request_ready_to_be_sent() {
        let request = to_request(
            "POST https://api.example.com/users\n\
             Content-Type: application/json\n\n\
             {\"name\": \"John\"}",
        )
        .unwrap();

        assert!(request.validate_json_body().is_ok());
    }

    /// Sorted so the comparison does not depend on `HeaderMap` iteration order,
    /// which is not insertion order.
    fn headers_of(request: &Request) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = request
            .headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();

        pairs.sort();

        pairs
    }

    fn assert_same_request(left: &Request, right: &Request) {
        assert_eq!(left.method, right.method);
        assert_eq!(left.url.as_str(), right.url.as_str());
        assert_eq!(left.body, right.body);
        assert_eq!(headers_of(left), headers_of(right));
    }

    fn assert_round_trips(source: &str) {
        let first = to_request(source).unwrap();
        let text = super::to_string(&first).unwrap();
        let second = to_request(&text).unwrap();

        assert_same_request(&first, &second);
    }

    /// The round trip is the gate for everything that writes to disk. If
    /// serialising loses a body newline, a header, or reorders them, `--save`
    /// would silently corrupt a user's file on every use.
    #[test]
    fn round_trips_every_request_shape() {
        let sources = [
            "GET https://api.example.com",
            "GET https://api.example.com/users?page=1&limit=10",
            "GET https://api.example.com/users#top",
            "POST https://api.example.com/users\nContent-Type: application/json",
            "POST https://api.example.com/users\nAccept: application/json\n\
             Authorization: Bearer abc123\nX-Trace: 42",
            "GET https://api.example.com\nAccept: application/json\n\
             Accept: text/plain",
            "POST https://api.example.com/users\nContent-Type: application/json\n\n\
             {\"name\": \"John\"}",
            "POST https://api.example.com/users\nContent-Type: text/plain\n\nline one\n\nline three",
            "POST https://api.example.com/users\nContent-Type: text/plain\n\n\nbody after a blank line",
            "POST https://api.example.com/users\nContent-Type: text/plain\n\ntrailing spaces   ",
            "PUT http://127.0.0.1:8080/users",
            "PATCH https://api.example.com\nContent-Length: 0",
            "DELETE https://api.example.com/users/42",
            "POST https://api.example.com/u\nContent-Type: text/plain\n\nbody\n\n",
            "POST https://api.example.com/u\nContent-Type: text/plain\n\n\n\nbody\n",
            "POST https://api.example.com/u\nContent-Type: text/plain\n\n\n",
            "POST https://api.example.com/u\nContent-Type: text/plain\n\n   ",
            "GET https://api.example.com\nAuthorization: Bearer",
            "GET https://api.example.com\nAuthorization: Basic",
            "GET https://api.example.com\nX-API-Key:",
        ];

        for source in sources {
            assert_round_trips(source);
        }
    }

    #[test]
    fn serialises_a_request_without_a_body() {
        let request = to_request("GET https://api.example.com/users").unwrap();

        let text = super::to_string(&request).unwrap();

        assert_eq!(text, "GET https://api.example.com/users\n");
    }

    #[test]
    fn serialises_a_request_with_a_body() {
        let request = to_request("POST https://api.example.com/users\n\n{\"a\": 1}").unwrap();

        let text = super::to_string(&request).unwrap();

        assert_eq!(text, "POST https://api.example.com/users\n\n{\"a\": 1}\n");
    }

    #[test]
    fn serialises_keeps_the_body_multiline() {
        let request = to_request("POST https://api.example.com/users\n\n{\n  \"a\": 1\n}").unwrap();

        let text = super::to_string(&request).unwrap();

        assert_eq!(
            text,
            "POST https://api.example.com/users\n\n{\n  \"a\": 1\n}\n"
        );
    }

    #[test]
    fn serialises_every_method() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
            let request = to_request(&format!("{method} https://api.example.com")).unwrap();

            let text = super::to_string(&request).unwrap();

            assert!(text.starts_with(&format!("{method} ")), "{text}");
        }
    }

    #[test]
    fn serialised_output_is_stable() {
        let request = to_request(
            "POST https://api.example.com/users\n\
             Accept: application/json\n\
             Content-Type: application/json\n\n\
             {\"a\": 1}",
        )
        .unwrap();

        let once = super::to_string(&request).unwrap();
        let twice = super::to_string(&to_request(&once).unwrap()).unwrap();

        assert_eq!(once, twice);
    }

    #[test]
    fn serialising_rejects_a_non_utf8_header_value() {
        let mut request = to_request("GET https://api.example.com").unwrap();
        request.headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap(),
        );

        let result = super::to_string(&request);

        assert!(matches!(result, Err(AppError::InvalidHeader(_))));
    }

    #[test]
    fn the_template_is_a_valid_request() {
        let request = to_request(super::template()).unwrap();

        assert_eq!(request.method, Method::GET);
        assert_eq!(request.url.as_str(), "https://api.example.com/");
        assert!(request.body.is_none());
    }

    #[test]
    fn the_template_round_trips() {
        let first = to_request(super::template()).unwrap();
        let text = super::to_string(&first).unwrap();
        let second = to_request(&text).unwrap();

        assert_same_request(&first, &second);
    }

    #[test]
    fn recognises_a_request_path_by_its_extension() {
        assert!(is_request_path("users/list.http"));
        assert!(is_request_path("users/admin/delete.http"));
        assert!(is_request_path("list.http"));
    }

    #[test]
    fn recognises_a_collection_path_by_its_absence_of_extension() {
        assert!(!is_request_path("users"));
        assert!(!is_request_path("users/admin"));
        assert!(!is_request_path("users/notes.txt"));
    }
}
