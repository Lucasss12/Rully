#[derive(Debug)]
pub enum AppError {
    ClientBuild(String),
    InvalidUrl(String),
    InvalidQuery(String),
    InvalidHeader(String),
    ConflictingBodyOptions,
    FileRead(std::io::Error),
    InvalidJson(serde_json::Error),
    Network(String),
    Timeout,
    ResponseBody(String),
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
            AppError::InvalidJson(_) => ErrorCategory::Cli,
            AppError::Network(_) => ErrorCategory::Network,
            AppError::Timeout => ErrorCategory::Network,
            AppError::ResponseBody(_) => ErrorCategory::Response,
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
            (AppError::Timeout, ErrorCategory::Network),
            (
                AppError::ResponseBody(String::new()),
                ErrorCategory::Response,
            ),
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
}
