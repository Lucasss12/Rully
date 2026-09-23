#[derive(Debug)]
pub enum AppError {
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
            AppError::InvalidUrl(message) => {
                write!(formatter, "URL invalide : {message}")
            }
            AppError::InvalidQuery(message) => {
                write!(formatter, "Requête invalide : {message}")
            }
            AppError::InvalidHeader(message) => {
                write!(formatter, "En-tête invalide : {message}")
            }
            AppError::ConflictingBodyOptions => {
                write!(formatter, "Options de corps en conflit")
            }
            AppError::FileRead(error) => {
                write!(formatter, "Impossible de lire le fichier : {error}")
            }
            AppError::InvalidJson(error) => {
                write!(formatter, "JSON invalide : {error}")
            }
            AppError::Network(error) => {
                write!(formatter, "Erreur réseau : {error}")
            }
            AppError::Timeout => {
                write!(formatter, "Délai de la requête dépassé")
            }
            AppError::ResponseBody(error) => {
                write!(formatter, "Erreur de corps de réponse : {error}")
            }
        }
    }
}
