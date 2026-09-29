use clap::{Parser, ValueEnum};
use reqwest::header::{HeaderName, HeaderValue};
use rully_core::errors::AppError;
use rully_core::request::Request;

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq)]
pub(crate) enum Method {
    #[value(name = "GET")]
    Get,
    #[value(name = "POST")]
    Post,
    #[value(name = "PUT")]
    Put,
    #[value(name = "PATCH")]
    Patch,
    #[value(name = "DELETE")]
    Delete,
}

/// Rully, a terminal-first HTTP client for the command line.
///
/// Send HTTP requests and inspect the response directly in the terminal.
#[derive(Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// HTTP method to use for the request.
    pub(crate) method: Method,

    /// Target URL of the request.
    pub(crate) url: String,

    /// Set a request header as `Name: value`. Can be repeated.
    #[arg(long = "header", value_name = "NAME: VALUE")]
    pub(crate) headers: Vec<String>,

    /// Add a query parameter as `name=value`. Can be repeated.
    #[arg(long = "query", value_name = "NAME=VALUE")]
    pub(crate) queries: Vec<String>,

    /// Send a raw request body as a string.
    #[arg(long)]
    pub(crate) body: Option<String>,

    /// Read the request body from a file.
    #[arg(long, value_name = "PATH")]
    pub(crate) body_file: Option<String>,

    /// Print verbose output with request and response headers.
    #[arg(short, long)]
    pub(crate) verbose: bool,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

impl TryFrom<&Cli> for Request {
    type Error = AppError;

    fn try_from(cli: &Cli) -> Result<Self, AppError> {
        let method = match cli.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Delete => reqwest::Method::DELETE,
            Method::Patch => reqwest::Method::PATCH,
        };

        let mut request = Request::new(method, &cli.url)?;

        add_query(&mut request, &cli.queries)?;
        add_headers(&mut request, &cli.headers)?;
        request.body = resolve_body(cli)?;

        Ok(request)
    }
}

fn add_query(request: &mut Request, queries: &[String]) -> Result<(), AppError> {
    if queries.is_empty() {
        return Ok(());
    }

    let mut pairs: Vec<(&str, &str)> = Vec::new();

    for query in queries {
        let (name, value) = query
            .split_once('=')
            .ok_or_else(|| AppError::InvalidQuery("Expected format: name=value".to_string()))?;

        let already_used = request
            .url
            .query_pairs()
            .any(|(existing, _)| existing == name)
            || pairs.iter().any(|(existing, _)| *existing == name);

        if already_used {
            return Err(AppError::InvalidQuery(format!(
                "Parameter already present: {name}"
            )));
        }

        pairs.push((name, value));
    }

    let mut serializer = request.url.query_pairs_mut();
    for (name, value) in pairs {
        serializer.append_pair(name, value);
    }
    drop(serializer);

    Ok(())
}

fn add_headers(request: &mut Request, headers: &[String]) -> Result<(), AppError> {
    for header in headers {
        let (name, value) = header
            .split_once(':')
            .ok_or_else(|| AppError::InvalidHeader("Expected format: Name: value".to_string()))?;

        let name = HeaderName::from_bytes(name.trim().as_bytes()).map_err(|_| {
            AppError::InvalidHeader(format!("Invalid header name: {}", name.trim()))
        })?;

        let value = HeaderValue::from_str(value.trim())
            .map_err(|_| AppError::InvalidHeader(format!("Invalid header value: {name}")))?;

        request.headers.append(name, value);
    }

    Ok(())
}

fn resolve_body(cli: &Cli) -> Result<Option<String>, AppError> {
    match (cli.body.as_deref(), cli.body_file.as_deref()) {
        (Some(_), Some(_)) => Err(AppError::ConflictingBodyOptions),
        (Some(body), None) => Ok(Some(body.to_owned())),
        (None, Some(path)) => Ok(Some(
            std::fs::read_to_string(path).map_err(AppError::FileRead)?,
        )),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(args: &[&str]) -> Result<Request, AppError> {
        let cli = Cli::try_parse_from(args.iter().copied()).unwrap();
        Request::try_from(&cli)
    }

    #[test]
    fn rejects_missing_required_arguments() {
        let result = Cli::try_parse_from(["rully"]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_url() {
        let result = Cli::try_parse_from(["rully", "GET"]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_method() {
        let result = Cli::try_parse_from(["rully", "https://example.com"]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_method() {
        let result = Cli::try_parse_from(["rully", "OPTIONS", "https://example.com"]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_default_arguments() {
        let cli = Cli::try_parse_from(["rully", "GET", "https://example.com"]).unwrap();
        assert_eq!(cli.method, Method::Get);
        assert_eq!(cli.url, "https://example.com");
        assert!(cli.headers.is_empty());
        assert!(cli.queries.is_empty());
        assert!(cli.body.is_none());
        assert!(!cli.verbose);
    }

    #[test]
    fn parses_all_supported_methods() {
        let methods = [
            ("GET", Method::Get),
            ("POST", Method::Post),
            ("PUT", Method::Put),
            ("PATCH", Method::Patch),
            ("DELETE", Method::Delete),
        ];
        for (input, expected) in methods {
            let cli = Cli::try_parse_from(["rully", input, "https://example.com"]).unwrap();
            assert_eq!(cli.method, expected);
        }
    }

    #[test]
    fn parses_multiple_headers() {
        let cli = Cli::try_parse_from([
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Authorization: Bearer token",
            "--header",
            "Content-Type: application/json",
        ])
        .unwrap();
        assert_eq!(cli.headers.len(), 2);
        assert_eq!(cli.headers[0], "Authorization: Bearer token");
        assert_eq!(cli.headers[1], "Content-Type: application/json");
    }

    #[test]
    fn parses_multiple_queries() {
        let cli = Cli::try_parse_from([
            "rully",
            "GET",
            "https://example.com",
            "--query",
            "page=1",
            "--query",
            "limit=10",
        ])
        .unwrap();
        assert_eq!(cli.queries, vec!["page=1", "limit=10"]);
    }

    #[test]
    fn parses_body() {
        let cli = Cli::try_parse_from([
            "rully",
            "POST",
            "https://example.com",
            "--body",
            r#"{"name":"John"}"#,
        ])
        .unwrap();
        assert_eq!(cli.body.as_deref(), Some(r#"{"name":"John"}"#));
    }

    #[test]
    fn parses_verbose_flags() {
        let long =
            Cli::try_parse_from(["rully", "GET", "https://example.com", "--verbose"]).unwrap();
        let short = Cli::try_parse_from(["rully", "GET", "https://example.com", "-v"]).unwrap();
        assert!(long.verbose);
        assert!(short.verbose);
    }

    #[test]
    fn converts_method_and_url() {
        let request = convert(&["rully", "PATCH", "https://example.com/users"]).unwrap();

        assert_eq!(request.method, reqwest::Method::PATCH);
        assert_eq!(request.url.as_str(), "https://example.com/users");
        assert!(request.body.is_none());
    }

    #[test]
    fn converts_invalid_url() {
        let result = convert(&["rully", "GET", "not-a-url"]);

        assert!(matches!(result, Err(AppError::InvalidUrl(_))));
    }

    #[test]
    fn converts_headers() {
        let request = convert(&[
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Authorization: Bearer token",
            "--header",
            "Content-Type: application/json",
        ])
        .unwrap();

        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.headers["authorization"], "Bearer token");
        assert_eq!(request.headers["content-type"], "application/json");
    }

    #[test]
    fn converts_header_without_spaces() {
        let request = convert(&[
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Accept:   text/plain  ",
        ])
        .unwrap();

        assert_eq!(request.headers["accept"], "text/plain");
    }

    #[test]
    fn keeps_duplicate_header_names() {
        let request = convert(&[
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Accept: application/json",
            "--header",
            "Accept: text/plain",
        ])
        .unwrap();

        let values: Vec<_> = request.headers.get_all("accept").iter().collect();

        assert_eq!(values.len(), 2);
    }

    #[test]
    fn converts_header_without_colon() {
        let result = convert(&[
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Accept text/plain",
        ]);

        assert!(matches!(result, Err(AppError::InvalidHeader(_))));
    }

    #[test]
    fn converts_invalid_header_name() {
        let result = convert(&[
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Bad Name: text/plain",
        ]);

        assert!(matches!(result, Err(AppError::InvalidHeader(_))));
    }

    #[test]
    fn converts_invalid_header_value() {
        let result = convert(&[
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Accept: text/plain\nX-Injected: yes",
        ]);

        assert!(matches!(result, Err(AppError::InvalidHeader(_))));
    }

    #[test]
    fn keeps_query_from_url() {
        let request =
            convert(&["rully", "GET", "https://example.com/search?q=rust&page=2"]).unwrap();

        assert_eq!(
            request.url.as_str(),
            "https://example.com/search?q=rust&page=2"
        );
    }

    #[test]
    fn merges_query_from_url_and_arguments() {
        let request = convert(&[
            "rully",
            "GET",
            "https://example.com/search?q=rust",
            "--query",
            "page=2",
        ])
        .unwrap();

        assert_eq!(
            request.url.as_str(),
            "https://example.com/search?q=rust&page=2"
        );
    }

    #[test]
    fn converts_query_arguments() {
        let request = convert(&[
            "rully",
            "GET",
            "https://example.com",
            "--query",
            "page=1",
            "--query",
            "limit=10",
        ])
        .unwrap();

        assert_eq!(request.url.as_str(), "https://example.com/?page=1&limit=10");
    }

    #[test]
    fn converts_query_without_equal_sign() {
        let result = convert(&["rully", "GET", "https://example.com", "--query", "page"]);

        assert!(matches!(result, Err(AppError::InvalidQuery(_))));
    }

    #[test]
    fn converts_query_already_in_url() {
        let result = convert(&[
            "rully",
            "GET",
            "https://example.com?page=1",
            "--query",
            "page=2",
        ]);

        assert!(matches!(result, Err(AppError::InvalidQuery(_))));
    }

    #[test]
    fn converts_duplicate_query_arguments() {
        let result = convert(&[
            "rully",
            "GET",
            "https://example.com",
            "--query",
            "page=1",
            "--query",
            "page=2",
        ]);

        assert!(matches!(result, Err(AppError::InvalidQuery(_))));
    }

    #[test]
    fn converts_raw_body() {
        let request = convert(&[
            "rully",
            "POST",
            "https://example.com",
            "--body",
            r#"{"name":"John"}"#,
        ])
        .unwrap();

        assert_eq!(request.body.as_deref(), Some(r#"{"name":"John"}"#));
    }

    #[test]
    fn converts_body_from_file() {
        let request = convert(&[
            "rully",
            "POST",
            "https://example.com",
            "--body-file",
            "test.json",
        ])
        .unwrap();

        let body = request.body.unwrap();

        assert!(body.contains("test@gmail.com"));
    }

    #[test]
    fn converts_conflicting_body_options() {
        let result = convert(&[
            "rully",
            "POST",
            "https://example.com",
            "--body",
            "{}",
            "--body-file",
            "test.json",
        ]);

        assert!(matches!(result, Err(AppError::ConflictingBodyOptions)));
    }

    #[test]
    fn converts_missing_body_file() {
        let result = convert(&[
            "rully",
            "POST",
            "https://example.com",
            "--body-file",
            "missing.json",
        ]);

        assert!(matches!(result, Err(AppError::FileRead(_))));
    }

    #[test]
    fn converts_json_request_ready_for_validation() {
        let request = convert(&[
            "rully",
            "POST",
            "https://example.com",
            "--header",
            "Content-Type: application/json",
            "--body",
            r#"{"name":"John"}"#,
        ])
        .unwrap();

        assert!(request.validate_json_body().is_ok());
    }
}
