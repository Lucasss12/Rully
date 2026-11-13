#[derive(Debug)]
pub enum AppError {
    ClientBuild(String),
    InvalidUrl(String),
    InvalidHeader(String),
    FileRead(std::io::Error),
    FileWrite(std::io::Error),
    InvalidJson(serde_json::Error),
    Network(String),
    Timeout,
    ResponseBody(String),
    InvalidHttpFile(String),
    InvalidEnvFile(String),
    UnknownVariable(String),
    EnvironmentNotFound(String),
    EnvironmentAlreadyExists(String),
    HttpVersionUnsupported { found: u32, supported: u32 },
    WorkspaceNotFound,
    CollectionNotFound(String),
    MissingCollection(String),
    CollectionAlreadyExists(String),
    CollectionNotEmpty(String),
    WorkspaceAlreadyExists,
    RequestNotFound(String),
    RequestAlreadyExists(String),
    InvalidRequestName(String),
    Terminal(std::io::Error),
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
            AppError::InvalidHeader(message) => {
                write!(formatter, "Invalid header: {message}")
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
            AppError::InvalidEnvFile(message) => {
                write!(formatter, "Invalid environment file: {message}")
            }
            AppError::UnknownVariable(name) => {
                write!(formatter, "Unknown variable \"{name}\"")
            }
            AppError::EnvironmentNotFound(name) => {
                write!(formatter, "Environment not found: {name}")
            }
            AppError::EnvironmentAlreadyExists(name) => {
                write!(formatter, "Environment already exists: {name}")
            }
            AppError::HttpVersionUnsupported { found, supported } => write!(
                formatter,
                "Format version {found} is not supported, rully supports up to {supported}"
            ),
            AppError::WorkspaceNotFound => write!(
                formatter,
                "No rully workspace found in the current directory"
            ),
            AppError::CollectionNotFound(name) => {
                write!(formatter, "Collection not found: {name}")
            }
            AppError::MissingCollection(name) => {
                write!(formatter, "Collection not found: {name}")
            }
            AppError::CollectionAlreadyExists(name) => {
                write!(formatter, "Collection already exists: {name}")
            }
            AppError::CollectionNotEmpty(name) => {
                write!(formatter, "Collection is not empty: {name}")
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
            AppError::Terminal(error) => {
                write!(formatter, "Could not write to the terminal: {error}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "No rully workspace found in the current directory"
        );
    }

    #[test]
    fn reports_a_missing_collection() {
        let error = AppError::CollectionNotFound("users".to_string());

        assert_eq!(error.to_string(), "Collection not found: users");
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
    fn formats_an_existing_collection() {
        let error = AppError::CollectionAlreadyExists("users".to_string());

        assert_eq!(error.to_string(), "Collection already exists: users");
    }

    #[test]
    fn formats_a_non_empty_collection() {
        let error = AppError::CollectionNotEmpty("users".to_string());

        assert_eq!(error.to_string(), "Collection is not empty: users");
    }

    #[test]
    fn formats_an_existing_workspace() {
        assert_eq!(
            AppError::WorkspaceAlreadyExists.to_string(),
            "A rully workspace already exists here"
        );
    }
}
