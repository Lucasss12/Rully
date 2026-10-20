use std::borrow::Cow;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use reqwest::header::{CONTENT_TYPE, HeaderMap};

pub(super) fn header_lines(headers: &HeaderMap) -> Vec<Line<'_>> {
    headers
        .iter()
        .map(|(name, value)| {
            let name = name.as_str();
            let value = value.to_str().unwrap_or("[non-UTF8]");

            Line::from(vec![
                Span::styled(format!("{name}:  "), Style::new().fg(Color::Yellow)),
                Span::raw(display_value(name, value)),
            ])
        })
        .collect()
}

pub(super) fn display_value<'a>(name: &str, value: &'a str) -> Cow<'a, str> {
    if is_sensitive(name) {
        Cow::Borrowed("[redacted]")
    } else {
        Cow::Borrowed(value)
    }
}

pub(super) fn is_sensitive(name: &str) -> bool {
    const SENSITIVE: [&str; 5] = [
        "authorization",
        "cookie",
        "set-cookie",
        "proxy-authorization",
        "x-api-key",
    ];

    SENSITIVE
        .iter()
        .any(|header| name.eq_ignore_ascii_case(header))
}

pub(super) fn body_text<'a>(body: &'a str, json: bool) -> Text<'a> {
    if json && let Some(pretty) = pretty_json(body) {
        return Text::from(pretty);
    }

    Text::from(body)
}

pub(super) fn pretty_json(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    serde_json::to_string_pretty(&value).ok()
}

pub(super) fn wants_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();

        for (name, value) in pairs {
            headers.append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }

        headers
    }

    #[test]
    fn lists_each_header_with_its_value() {
        let headers = headers(&[("accept", "application/json")]);

        let lines = header_lines(&headers);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "accept:  application/json");
    }

    #[test]
    fn keeps_duplicate_headers() {
        let headers = headers(&[("accept", "application/json"), ("accept", "text/plain")]);

        let lines = header_lines(&headers);

        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn redacts_sensitive_headers() {
        let headers = headers(&[("authorization", "Bearer secret")]);

        let rendered = header_lines(&headers)[0].to_string();

        assert!(rendered.contains("[redacted]"), "{rendered}");
        assert!(!rendered.contains("secret"), "{rendered}");
    }

    #[test]
    fn matches_sensitive_headers_regardless_of_case() {
        assert!(is_sensitive("Authorization"));
        assert!(is_sensitive("AUTHORIZATION"));
    }

    #[test]
    fn keeps_regular_headers_visible() {
        assert!(!is_sensitive("accept"));
        assert!(!is_sensitive("content-type"));
    }

    #[test]
    fn marks_a_non_utf8_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-bytes", HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap());

        let rendered = header_lines(&headers)[0].to_string();

        assert!(rendered.contains("[non-UTF8]"), "{rendered}");
    }

    fn with_content_type(content_type: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_str(content_type).unwrap());
        headers
    }

    #[test]
    fn pretty_prints_a_json_object() {
        let pretty = pretty_json(r#"{"a":1,"b":2}"#).unwrap();

        assert!(pretty.contains('\n'), "{pretty}");
        assert!(pretty.contains("\"a\": 1"), "{pretty}");
    }

    #[test]
    fn refuses_to_pretty_print_a_non_json_body() {
        assert_eq!(pretty_json("hello"), None);
        assert_eq!(pretty_json(""), None);
    }

    #[test]
    fn detects_a_json_content_type() {
        assert!(wants_json(&with_content_type("application/json")));
        assert!(wants_json(&with_content_type(
            "application/json; charset=utf-8"
        )));
        assert!(wants_json(&with_content_type("APPLICATION/JSON")));
    }

    #[test]
    fn ignores_other_content_types() {
        assert!(!wants_json(&with_content_type("text/plain")));
        assert!(!wants_json(&HeaderMap::new()));
    }

    #[test]
    fn indents_a_json_body_when_the_content_type_says_so() {
        let text = body_text(r#"{"a":1}"#, true);

        assert!(text.lines.len() > 1, "{text:?}");
    }

    #[test]
    fn renders_a_text_body_verbatim() {
        let text = body_text("line one\nline two", false);

        assert_eq!(text.lines.len(), 2);
        assert_eq!(text.lines[0].to_string(), "line one");
        assert_eq!(text.lines[1].to_string(), "line two");
    }
}
