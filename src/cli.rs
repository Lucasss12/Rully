use clap::{Parser, Subcommand, ValueEnum};
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

#[derive(Debug, PartialEq, Subcommand)]
pub(crate) enum Command {
    /// Create the workspace in the current directory.
    Init,

    /// Create a collection, or a request inside an existing collection.
    New {
        /// Collection name, or `collection/request.http` for a request.
        name: String,
    },

    /// List collections, or the requests of a collection.
    List {
        /// List the requests of this collection instead.
        collection: Option<String>,
    },

    /// Execute a saved request.
    Run {
        /// Path of the request, relative to the collections directory.
        path: String,
    },

    /// Edit a saved request in your editor.
    Edit {
        /// Path of the request, relative to the collections directory.
        path: String,
    },

    /// Delete a saved request.
    Delete {
        /// Path of the request, relative to the collections directory.
        path: String,
    },
}

/// Rully, a terminal-first HTTP client for the command line.
///
/// Send HTTP requests and inspect the response directly in the terminal.
#[derive(Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,

    /// HTTP method to use for the request.
    pub(crate) method: Option<Method>,

    /// Target URL of the request.
    pub(crate) url: Option<String>,

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

    /// Save the request to the collections directory before sending it.
    #[arg(long, value_name = "COLLECTION/NAME")]
    pub(crate) save: Option<String>,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

impl Cli {
    /// Checks that the invocation is complete: either a subcommand, or both a
    /// method and a URL.
    pub(crate) fn validate(&self) -> Result<(), AppError> {
        match (&self.command, &self.method, &self.url) {
            (Some(_), _, _) => Ok(()),
            (None, Some(_), Some(_)) => Ok(()),
            (None, Some(_), None) => Err(AppError::MissingArguments(
                "expected a URL, or a subcommand such as `rully init`".to_string(),
            )),
            (None, None, Some(_)) => Err(AppError::MissingArguments(
                "expected a method, or a subcommand such as `rully init`".to_string(),
            )),
            (None, None, None) => Err(AppError::MissingArguments(
                "expected a method and a URL, or a subcommand such as `rully init`".to_string(),
            )),
        }
    }
}

impl TryFrom<&Cli> for Request {
    type Error = AppError;

    fn try_from(cli: &Cli) -> Result<Self, AppError> {
        let method = match cli.method {
            Some(Method::Get) => reqwest::Method::GET,
            Some(Method::Post) => reqwest::Method::POST,
            Some(Method::Put) => reqwest::Method::PUT,
            Some(Method::Delete) => reqwest::Method::DELETE,
            Some(Method::Patch) => reqwest::Method::PATCH,
            None => {
                return Err(AppError::MissingArguments("expected a method".to_string()));
            }
        };

        let url = cli
            .url
            .as_deref()
            .ok_or_else(|| AppError::MissingArguments("expected a URL".to_string()))?;

        let mut request = Request::new(method, url)?;

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
    fn rejects_missing_url() {
        let cli = Cli::try_parse_from(["rully", "GET"]).unwrap();
        let result = cli.validate();

        assert!(matches!(result, Err(AppError::MissingArguments(_))));
    }

    #[test]
    fn rejects_missing_required_arguments() {
        let cli = Cli::try_parse_from(["rully"]).unwrap();
        let result = cli.validate();

        assert!(matches!(result, Err(AppError::MissingArguments(_))));
    }

    #[test]
    fn rejects_a_url_without_a_method() {
        let result = Cli::try_parse_from(["rully", "https://example.com"]);

        assert!(result.is_err());
    }

    #[test]
    fn accepts_the_naked_form() {
        let cli = Cli::try_parse_from(["rully", "GET", "https://example.com"]).unwrap();

        assert!(cli.validate().is_ok());
    }

    #[test]
    fn parses_the_init_subcommand() {
        let cli = Cli::try_parse_from(["rully", "init"]).unwrap();

        assert_eq!(cli.command, Some(Command::Init));
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn parses_the_new_subcommand() {
        let cli = Cli::try_parse_from(["rully", "new", "users"]).unwrap();

        assert_eq!(
            cli.command,
            Some(Command::New {
                name: "users".to_string()
            })
        );
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn parses_the_list_subcommand_without_argument() {
        let cli = Cli::try_parse_from(["rully", "list"]).unwrap();

        assert_eq!(cli.command, Some(Command::List { collection: None }));
    }

    #[test]
    fn parses_the_list_subcommand_with_a_collection() {
        let cli = Cli::try_parse_from(["rully", "list", "users"]).unwrap();

        assert_eq!(
            cli.command,
            Some(Command::List {
                collection: Some("users".to_string())
            })
        );
    }

    #[test]
    fn parses_the_run_subcommand() {
        let cli = Cli::try_parse_from(["rully", "run", "users/list.http"]).unwrap();

        assert_eq!(
            cli.command,
            Some(Command::Run {
                path: "users/list.http".to_string()
            })
        );
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn run_requires_a_path() {
        let result = Cli::try_parse_from(["rully", "run"]);

        assert!(result.is_err());
    }

    #[test]
    fn new_requires_a_collection() {
        let result = Cli::try_parse_from(["rully", "new"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_an_unknown_subcommand() {
        let result = Cli::try_parse_from(["rully", "frobnicate", "https://example.com"]);

        assert!(result.is_err());
    }

    #[test]
    fn keeps_verbose_on_a_subcommand() {
        let cli = Cli::try_parse_from(["rully", "-v", "run", "users/list.http"]).unwrap();

        assert!(cli.verbose);
    }

    #[test]
    fn accepts_a_subcommand_without_a_method_and_url() {
        for args in [
            &["rully", "init"][..],
            &["rully", "new", "users"][..],
            &["rully", "list"][..],
            &["rully", "run", "users/list.http"][..],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();

            assert!(cli.validate().is_ok(), "{args:?}");
        }
    }

    #[test]
    fn converts_a_method_and_a_url_without_a_subcommand() {
        let cli = Cli::try_parse_from(["rully", "GET", "https://example.com"]).unwrap();
        let request = Request::try_from(&cli).unwrap();

        assert_eq!(request.method, reqwest::Method::GET);
        assert_eq!(request.url.as_str(), "https://example.com/");
    }

    #[test]
    fn rejects_a_conversion_without_a_method() {
        let result = Cli::try_parse_from(["rully", "https://example.com"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_a_conversion_without_a_url() {
        let cli = Cli::try_parse_from(["rully"]).unwrap();
        let result = Request::try_from(&cli);

        assert!(matches!(result, Err(AppError::MissingArguments(_))));
    }

    #[test]
    fn parses_the_edit_subcommand() {
        let cli = Cli::try_parse_from(["rully", "edit", "users/list.http"]).unwrap();

        assert_eq!(
            cli.command,
            Some(Command::Edit {
                path: "users/list.http".to_string()
            })
        );
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn parses_the_delete_subcommand() {
        let cli = Cli::try_parse_from(["rully", "delete", "users/list.http"]).unwrap();

        assert_eq!(
            cli.command,
            Some(Command::Delete {
                path: "users/list.http".to_string()
            })
        );
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn edit_requires_a_path() {
        assert!(Cli::try_parse_from(["rully", "edit"]).is_err());
    }

    #[test]
    fn delete_requires_a_path() {
        assert!(Cli::try_parse_from(["rully", "delete"]).is_err());
    }

    #[test]
    fn parses_a_request_path_in_new() {
        let cli = Cli::try_parse_from(["rully", "new", "users/list.http"]).unwrap();

        assert_eq!(
            cli.command,
            Some(Command::New {
                name: "users/list.http".to_string()
            })
        );
    }

    #[test]
    fn parses_a_collection_name_in_new() {
        let cli = Cli::try_parse_from(["rully", "new", "users"]).unwrap();

        assert_eq!(
            cli.command,
            Some(Command::New {
                name: "users".to_string()
            })
        );
    }

    #[test]
    fn parses_the_save_flag() {
        let cli = Cli::try_parse_from([
            "rully",
            "GET",
            "https://example.com",
            "--save",
            "users/list",
        ])
        .unwrap();

        assert_eq!(cli.save.as_deref(), Some("users/list"));
        assert_eq!(cli.method, Some(Method::Get));
    }

    #[test]
    fn the_save_flag_is_optional() {
        let cli = Cli::try_parse_from(["rully", "GET", "https://example.com"]).unwrap();

        assert!(cli.save.is_none());
    }

    #[test]
    fn the_save_flag_requires_a_value() {
        let result = Cli::try_parse_from(["rully", "GET", "https://example.com", "--save"]);

        assert!(result.is_err());
    }

    #[test]
    fn the_save_flag_is_not_accepted_on_a_subcommand() {
        let result =
            Cli::try_parse_from(["rully", "run", "users/list.http", "--save", "users/other"]);

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
        assert_eq!(cli.method, Some(Method::Get));
        assert_eq!(cli.url.as_deref(), Some("https://example.com"));
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
            assert_eq!(cli.method, Some(expected));
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
