#[derive(Debug)]
pub enum AppError {
    ClientBuild(String),
    InvalidUrl(String),
    InvalidQuery(String),
    InvalidHeader(String),
    ConflictingBodyOptions,
    FileRead(std::io::Error),
    FileWrite(std::io::Error),
    InvalidJson(serde_json::Error),
    Network(String),
    Timeout,
    ResponseBody(String),
    InvalidHttpFile(String),
    HttpVersionUnsupported { found: u32, supported: u32 },
    WorkspaceNotFound,
    CollectionNotFound(String),
    MissingCollection(String),
    CollectionAlreadyExists(String),
    WorkspaceAlreadyExists,
    RequestNotFound(String),
    RequestAlreadyExists(String),
    InvalidRequestName(String),
    MissingArguments(String),
    NoEditor,
    EditorFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorCategory {
    Cli,
    Url,
    Network,
    Response,
}

impl AppError {
    pub fn category(&self) -> ErrorCategory {
        match self {
            AppError::ClientBuild(_) => ErrorCategory::Network,
            AppError::InvalidUrl(_) => ErrorCategory::Url,
            AppError::InvalidQuery(_) => ErrorCategory::Cli,
            AppError::InvalidHeader(_) => ErrorCategory::Cli,
            AppError::ConflictingBodyOptions => ErrorCategory::Cli,
            AppError::FileRead(_) => ErrorCategory::Cli,
            AppError::FileWrite(_) => ErrorCategory::Cli,
            AppError::InvalidJson(_) => ErrorCategory::Cli,
            AppError::Network(_) => ErrorCategory::Network,
            AppError::Timeout => ErrorCategory::Network,
            AppError::ResponseBody(_) => ErrorCategory::Response,
            AppError::InvalidHttpFile(_) => ErrorCategory::Cli,
            AppError::HttpVersionUnsupported { .. } => ErrorCategory::Cli,
            AppError::WorkspaceNotFound => ErrorCategory::Cli,
            AppError::CollectionNotFound(_) => ErrorCategory::Cli,
            AppError::MissingCollection(_) => ErrorCategory::Cli,
            AppError::CollectionAlreadyExists(_) => ErrorCategory::Cli,
            AppError::WorkspaceAlreadyExists => ErrorCategory::Cli,
            AppError::RequestNotFound(_) => ErrorCategory::Cli,
            AppError::RequestAlreadyExists(_) => ErrorCategory::Cli,
            AppError::InvalidRequestName(_) => ErrorCategory::Cli,
            AppError::MissingArguments(_) => ErrorCategory::Cli,
            AppError::NoEditor => ErrorCategory::Cli,
            AppError::EditorFailed(_) => ErrorCategory::Cli,
        }
    }
}

impl std::error::Error for AppError {}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppError::ClientBuild(message) => {
                write!(formatter, "HTTP client error: {message}")
            }
            AppError::InvalidUrl(message) => {
                write!(formatter, "Invalid URL: {message}")
            }
            AppError::InvalidQuery(message) => {
                write!(formatter, "Invalid query: {message}")
            }
            AppError::InvalidHeader(message) => {
                write!(formatter, "Invalid header: {message}")
            }
            AppError::ConflictingBodyOptions => {
                write!(formatter, "Conflicting body options")
            }
            AppError::FileRead(error) => {
                write!(formatter, "Could not read file: {error}")
            }
            AppError::FileWrite(error) => {
                write!(formatter, "Could not write file: {error}")
            }
            AppError::InvalidJson(error) => {
                write!(formatter, "Invalid JSON: {error}")
            }
            AppError::Network(error) => {
                write!(formatter, "Network error: {error}")
            }
            AppError::Timeout => {
                write!(formatter, "Request timed out")
            }
            AppError::ResponseBody(error) => {
                write!(formatter, "Response body error: {error}")
            }
            AppError::InvalidHttpFile(message) => {
                write!(formatter, "Invalid request file: {message}")
            }
            AppError::HttpVersionUnsupported { found, supported } => write!(
                formatter,
                "Format version {found} is not supported, rully supports up to {supported}"
            ),
            AppError::WorkspaceNotFound => write!(
                formatter,
                "No rully workspace found, run `rully init` first"
            ),
            AppError::CollectionNotFound(name) => {
                write!(formatter, "Collection not found: {name}")
            }
            AppError::MissingCollection(name) => {
                write!(
                    formatter,
                    "Collection not found: {name}, create it with `rully new {name}`"
                )
            }
            AppError::CollectionAlreadyExists(name) => {
                write!(formatter, "Collection already exists: {name}")
            }
            AppError::WorkspaceAlreadyExists => {
                write!(formatter, "A rully workspace already exists here")
            }
            AppError::RequestNotFound(path) => {
                write!(formatter, "Request not found: {path}")
            }
            AppError::RequestAlreadyExists(path) => {
                write!(formatter, "Request already exists: {path}")
            }
            AppError::InvalidRequestName(message) => {
                write!(formatter, "Invalid request name: {message}")
            }
            AppError::MissingArguments(message) => {
                write!(formatter, "{message}")
            }
            AppError::NoEditor => write!(
                formatter,
                "No editor found, set $VISUAL or $EDITOR to edit this request"
            ),
            AppError::EditorFailed(message) => {
                write!(formatter, "Editor failed: {message}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_variant_to_a_category() {
        let cases: Vec<(AppError, ErrorCategory)> = vec![
            (AppError::ClientBuild(String::new()), ErrorCategory::Network),
            (AppError::InvalidUrl(String::new()), ErrorCategory::Url),
            (AppError::InvalidQuery(String::new()), ErrorCategory::Cli),
            (AppError::InvalidHeader(String::new()), ErrorCategory::Cli),
            (AppError::ConflictingBodyOptions, ErrorCategory::Cli),
            (
                AppError::FileRead(std::io::Error::other("disk")),
                ErrorCategory::Cli,
            ),
            (
                AppError::FileWrite(std::io::Error::other("disk")),
                ErrorCategory::Cli,
            ),
            (
                AppError::InvalidJson(serde_json::from_str::<serde_json::Value>("{").unwrap_err()),
                ErrorCategory::Cli,
            ),
            (AppError::Network(String::new()), ErrorCategory::Network),
            (AppError::Timeout, ErrorCategory::Network),
            (
                AppError::ResponseBody(String::new()),
                ErrorCategory::Response,
            ),
            (AppError::InvalidHttpFile(String::new()), ErrorCategory::Cli),
            (
                AppError::HttpVersionUnsupported {
                    found: 2,
                    supported: 1,
                },
                ErrorCategory::Cli,
            ),
            (AppError::WorkspaceNotFound, ErrorCategory::Cli),
            (
                AppError::CollectionNotFound(String::new()),
                ErrorCategory::Cli,
            ),
            (
                AppError::MissingCollection(String::new()),
                ErrorCategory::Cli,
            ),
            (
                AppError::CollectionAlreadyExists(String::new()),
                ErrorCategory::Cli,
            ),
            (AppError::WorkspaceAlreadyExists, ErrorCategory::Cli),
            (AppError::RequestNotFound(String::new()), ErrorCategory::Cli),
            (
                AppError::RequestAlreadyExists(String::new()),
                ErrorCategory::Cli,
            ),
            (
                AppError::InvalidRequestName(String::new()),
                ErrorCategory::Cli,
            ),
            (
                AppError::MissingArguments(String::new()),
                ErrorCategory::Cli,
            ),
            (AppError::NoEditor, ErrorCategory::Cli),
            (AppError::EditorFailed(String::new()), ErrorCategory::Cli),
        ];

        for (error, expected) in cases {
            assert_eq!(error.category(), expected, "{error}");
        }
    }

    #[test]
    fn formats_client_build_failure() {
        let error = AppError::ClientBuild("TLS backend unavailable".to_string());

        assert_eq!(
            error.to_string(),
            "HTTP client error: TLS backend unavailable"
        );
    }

    #[test]
    fn formats_file_write_failure() {
        let error = AppError::FileWrite(std::io::Error::other("read-only file system"));

        assert_eq!(
            error.to_string(),
            "Could not write file: read-only file system"
        );
    }

    #[test]
    fn formats_invalid_http_file() {
        let error = AppError::InvalidHttpFile("missing request line".to_string());

        assert_eq!(
            error.to_string(),
            "Invalid request file: missing request line"
        );
    }

    #[test]
    fn formats_unsupported_format_version() {
        let error = AppError::HttpVersionUnsupported {
            found: 3,
            supported: 1,
        };

        assert_eq!(
            error.to_string(),
            "Format version 3 is not supported, rully supports up to 1"
        );
    }

    #[test]
    fn formats_missing_workspace() {
        assert_eq!(
            AppError::WorkspaceNotFound.to_string(),
            "No rully workspace found, run `rully init` first"
        );
    }

    #[test]
    fn reports_a_missing_collection() {
        let error = AppError::CollectionNotFound("users".to_string());

        assert_eq!(error.to_string(), "Collection not found: users");
    }

    #[test]
    fn suggests_the_command_to_create_a_missing_collection() {
        let error = AppError::MissingCollection("users".to_string());

        assert_eq!(
            error.to_string(),
            "Collection not found: users, create it with `rully new users`"
        );
    }

    #[test]
    fn formats_missing_request() {
        let error = AppError::RequestNotFound("users/list.http".to_string());

        assert_eq!(error.to_string(), "Request not found: users/list.http");
    }

    #[test]
    fn formats_existing_request() {
        let error = AppError::RequestAlreadyExists("users/list.http".to_string());

        assert_eq!(error.to_string(), "Request already exists: users/list.http");
    }

    #[test]
    fn formats_invalid_request_name() {
        let error = AppError::InvalidRequestName("cannot contain < > : \"".to_string());

        assert_eq!(
            error.to_string(),
            "Invalid request name: cannot contain < > : \""
        );
    }

    #[test]
    fn formats_missing_arguments() {
        let error = AppError::MissingArguments("expected a method and a URL".to_string());

        assert_eq!(error.to_string(), "expected a method and a URL");
    }

    #[test]
    fn formats_a_missing_editor() {
        assert_eq!(
            AppError::NoEditor.to_string(),
            "No editor found, set $VISUAL or $EDITOR to edit this request"
        );
    }

    #[test]
    fn formats_a_failed_editor() {
        let error = AppError::EditorFailed("vim exited with status 3".to_string());

        assert_eq!(error.to_string(), "Editor failed: vim exited with status 3");
    }

    #[test]
    fn formats_an_existing_collection() {
        let error = AppError::CollectionAlreadyExists("users".to_string());

        assert_eq!(error.to_string(), "Collection already exists: users");
    }

    #[test]
    fn formats_an_existing_workspace() {
        assert_eq!(
            AppError::WorkspaceAlreadyExists.to_string(),
            "A rully workspace already exists here"
        );
    }
}
