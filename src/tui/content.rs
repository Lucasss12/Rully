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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyKind {
    None,
    Json,
    Text,
    Form,
}

/// The body kinds in the order the type menu lists them.
pub(super) const BODY_KINDS: [BodyKind; 4] = [
    BodyKind::None,
    BodyKind::Json,
    BodyKind::Text,
    BodyKind::Form,
];

impl BodyKind {
    /// Classifies a body from what the wire already knows: its presence and
    /// its `Content-Type`. The type is never stored in the file.
    pub(super) fn of(headers: &HeaderMap, body: Option<&str>) -> Self {
        if body.is_none() {
            return Self::None;
        }

        let media_type = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(str::trim);

        match media_type {
            Some(media_type) if media_type.eq_ignore_ascii_case("application/json") => Self::Json,
            Some(media_type)
                if media_type.eq_ignore_ascii_case("application/x-www-form-urlencoded") =>
            {
                Self::Form
            }
            _ => Self::Text,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Json => "json",
            Self::Text => "text",
            Self::Form => "form",
        }
    }

    pub(super) fn content_type(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Json => Some("application/json"),
            Self::Text => Some("text/plain"),
            Self::Form => Some("application/x-www-form-urlencoded"),
        }
    }

    pub(super) fn index(self) -> usize {
        match self {
            Self::None => 0,
            Self::Json => 1,
            Self::Text => 2,
            Self::Form => 3,
        }
    }
}

pub(super) fn body_text<'a>(body: &'a str, kind: BodyKind) -> Text<'a> {
    match kind {
        BodyKind::Json => match pretty_json(body) {
            Some(pretty) => Text::from(pretty),
            None => Text::from(body),
        },
        BodyKind::Form => Text::from(form_lines(body)),
        BodyKind::Text => Text::from(body),
        BodyKind::None => Text::default(),
    }
}

pub(super) fn pretty_json(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    serde_json::to_string_pretty(&value).ok()
}

/// Renders a urlencoded body as one key/value line per pair.
///
/// Values are shown exactly as they are sent (still percent-encoded): the TUI
/// is a debugging tool, so it should not hide what is on the wire.
pub(super) fn form_lines(body: &str) -> Vec<Line<'_>> {
    body.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((name, value)) => Line::from(vec![
                Span::styled(name.to_string(), Style::new().fg(Color::Yellow)),
                Span::styled(" = ", Style::new().fg(Color::DarkGray)),
                Span::raw(value),
            ]),
            None => Line::from(pair.to_string()),
        })
        .collect()
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
    fn detects_a_json_body() {
        for content_type in [
            "application/json",
            "application/json; charset=utf-8",
            "APPLICATION/JSON",
        ] {
            let headers = with_content_type(content_type);

            assert_eq!(
                BodyKind::of(&headers, Some(r#"{"a":1}"#)),
                BodyKind::Json,
                "{content_type}"
            );
        }
    }

    #[test]
    fn detects_a_form_body() {
        let headers = with_content_type("application/x-www-form-urlencoded");

        assert_eq!(BodyKind::of(&headers, Some("a=1")), BodyKind::Form);
    }

    #[test]
    fn falls_back_to_text_for_other_content_types() {
        let headers = with_content_type("text/plain");
        assert_eq!(BodyKind::of(&headers, Some("hello")), BodyKind::Text);

        let headers = with_content_type("application/xml");
        assert_eq!(BodyKind::of(&headers, Some("<a/>")), BodyKind::Text);

        assert_eq!(
            BodyKind::of(&HeaderMap::new(), Some("hello")),
            BodyKind::Text
        );
    }

    #[test]
    fn has_no_body_without_content() {
        let headers = with_content_type("application/json");

        assert_eq!(BodyKind::of(&headers, None), BodyKind::None);
        assert_eq!(BodyKind::of(&HeaderMap::new(), None), BodyKind::None);
    }

    #[test]
    fn maps_each_kind_to_its_label_and_content_type() {
        assert_eq!(BodyKind::None.label(), "none");
        assert_eq!(BodyKind::None.content_type(), None);
        assert_eq!(BodyKind::Json.label(), "json");
        assert_eq!(BodyKind::Json.content_type(), Some("application/json"));
        assert_eq!(BodyKind::Text.label(), "text");
        assert_eq!(BodyKind::Text.content_type(), Some("text/plain"));
        assert_eq!(BodyKind::Form.label(), "form");
        assert_eq!(
            BodyKind::Form.content_type(),
            Some("application/x-www-form-urlencoded")
        );
    }

    #[test]
    fn indexes_the_kinds_in_menu_order() {
        for (index, kind) in BODY_KINDS.iter().enumerate() {
            assert_eq!(kind.index(), index);
        }
    }

    #[test]
    fn lists_form_pairs() {
        let lines = form_lines("name=John&age=30");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].to_string(), "name = John");
        assert_eq!(lines[1].to_string(), "age = 30");
    }

    #[test]
    fn keeps_form_pairs_without_a_value() {
        let lines = form_lines("flag&name=John&");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].to_string(), "flag");
        assert_eq!(lines[1].to_string(), "name = John");
    }

    #[test]
    fn keeps_form_values_percent_encoded() {
        let lines = form_lines("name=John%20Doe");

        assert_eq!(lines[0].to_string(), "name = John%20Doe");
    }

    #[test]
    fn indents_a_json_body_when_the_kind_says_so() {
        let text = body_text(r#"{"a":1}"#, BodyKind::Json);

        assert!(text.lines.len() > 1, "{text:?}");
    }

    #[test]
    fn renders_a_text_body_verbatim() {
        let text = body_text("line one\nline two", BodyKind::Text);

        assert_eq!(text.lines.len(), 2);
        assert_eq!(text.lines[0].to_string(), "line one");
        assert_eq!(text.lines[1].to_string(), "line two");
    }

    #[test]
    fn renders_a_form_body_as_pairs() {
        let text = body_text("name=John&age=30", BodyKind::Form);

        assert_eq!(text.lines.len(), 2);
        assert_eq!(text.lines[0].to_string(), "name = John");
    }

    #[test]
    fn renders_nothing_for_the_none_kind() {
        let text = body_text("kept in the draft", BodyKind::None);

        assert!(text.lines.is_empty());
    }
}
