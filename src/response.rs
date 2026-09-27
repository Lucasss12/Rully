use reqwest::StatusCode;
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use std::time::Duration;

pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
    pub size: usize,
    pub duration: Duration,
}

impl Response {
    pub fn new(
        status: StatusCode,
        headers: HeaderMap,
        bytes: impl Into<Vec<u8>>,
        duration: Duration,
    ) -> Self {
        let bytes = bytes.into();
        let size = bytes.len();

        let body = match String::from_utf8(bytes) {
            Ok(body) => body,
            Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
        };

        Self {
            status,
            headers,
            body,
            size,
            duration,
        }
    }

    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.split(';').next().unwrap_or("").trim())
            .filter(|value| !value.is_empty())
    }

    pub fn duration_ms(&self) -> f64 {
        self.duration.as_secs_f64() * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    fn response(headers: HeaderMap, bytes: &[u8]) -> Response {
        Response::new(
            StatusCode::OK,
            headers,
            bytes.to_vec(),
            Duration::from_millis(142),
        )
    }

    #[test]
    fn keeps_status_body_and_size() {
        let response = response(HeaderMap::new(), b"hello");

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, "hello");
        assert_eq!(response.size, 5);
    }

    #[test]
    fn counts_bytes_and_not_characters() {
        let response = response(HeaderMap::new(), "héllo".as_bytes());

        assert_eq!(response.body.chars().count(), 5);
        assert_eq!(response.size, 6);
    }

    #[test]
    fn replaces_invalid_utf8_bytes() {
        let response = response(HeaderMap::new(), &[0xFF, 0xFE]);

        assert_eq!(response.size, 2);
        assert_eq!(response.body, "\u{FFFD}\u{FFFD}");
    }

    #[test]
    fn reads_content_type_without_charset() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        let response = response(headers, b"hello");

        assert_eq!(response.content_type(), Some("application/json"));
    }

    #[test]
    fn ignores_missing_and_empty_content_type() {
        assert_eq!(response(HeaderMap::new(), b"hello").content_type(), None);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(""));
        assert_eq!(response(headers, b"hello").content_type(), None);
    }

    #[test]
    fn converts_duration_to_milliseconds() {
        let response = Response::new(
            StatusCode::OK,
            HeaderMap::new(),
            b"hello".to_vec(),
            Duration::from_micros(142_500),
        );

        assert_eq!(response.duration_ms(), 142.5);
    }
}
