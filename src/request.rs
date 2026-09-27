use crate::errors::AppError;
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use reqwest::{Method, Url};

pub struct Request {
    pub method: Method,
    pub url: Url,
    pub headers: HeaderMap,
    pub body: Option<String>,
}

impl Request {
    pub fn new(method: Method, url: &str) -> Result<Self, AppError> {
        let url = Url::parse(url).map_err(|error| AppError::InvalidUrl(error.to_string()))?;

        Ok(Self {
            method,
            url,
            headers: HeaderMap::new(),
            body: None,
        })
    }

    fn is_json(&self) -> bool {
        self.headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("application/json"))
    }

    pub fn validate_json_body(&self) -> Result<(), AppError> {
        if !self.is_json() {
            return Ok(());
        }

        let Some(body) = &self.body else {
            return Ok(());
        };

        serde_json::from_str::<serde_json::Value>(body).map_err(AppError::InvalidJson)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    fn json_request() -> Request {
        let mut request = Request::new(Method::POST, "https://example.com").unwrap();
        request.headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str("application/json").unwrap(),
        );
        request
    }

    #[test]
    fn rejects_invalid_url() {
        let result = Request::new(Method::GET, "not-an-url");

        assert!(matches!(result, Err(AppError::InvalidUrl(_))));
    }

    #[test]
    fn starts_empty() {
        let request = Request::new(Method::GET, "https://example.com").unwrap();

        assert_eq!(request.method, Method::GET);
        assert_eq!(request.url.as_str(), "https://example.com/");
        assert!(request.headers.is_empty());
        assert!(request.body.is_none());
    }

    #[test]
    fn detects_json_content_type() {
        for content_type in [
            "application/json",
            "application/json; charset=utf-8",
            "APPLICATION/JSON",
        ] {
            let mut request = json_request();
            request
                .headers
                .insert(CONTENT_TYPE, HeaderValue::from_str(content_type).unwrap());

            assert!(request.is_json(), "{content_type} should be json");
        }
    }

    #[test]
    fn is_not_json_with_another_content_type() {
        let mut request = json_request();
        request
            .headers
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));

        assert!(!request.is_json());
    }

    #[test]
    fn is_not_json_without_content_type() {
        let request = Request::new(Method::POST, "https://example.com").unwrap();

        assert!(!request.is_json());
    }

    #[test]
    fn accepts_valid_json_body() {
        let mut request = json_request();
        request.body = Some(r#"{"name":"John"}"#.to_string());

        assert!(request.validate_json_body().is_ok());
    }

    #[test]
    fn rejects_invalid_json_body() {
        let mut request = json_request();
        request.body = Some("{invalid}".to_string());

        assert!(matches!(
            request.validate_json_body(),
            Err(AppError::InvalidJson(_))
        ));
    }

    #[test]
    fn ignores_body_without_json_content_type() {
        let mut request = json_request();
        request
            .headers
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        request.body = Some("{invalid}".to_string());

        assert!(request.validate_json_body().is_ok());
    }

    #[test]
    fn accepts_json_content_type_without_body() {
        let request = json_request();

        assert!(request.validate_json_body().is_ok());
    }
}
