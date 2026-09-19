#[derive(Debug)]
pub enum AppError {
    InvalidUrl(String),
    InvalidQuery(String),
    InvalidHeader(String),
    ConflictingBodyOptions,
    FileRead(std::io::Error),
    InvalidJson(serde_json::Error),
    Network(reqwest::Error),
    ResponseBody(reqwest::Error),
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
            AppError::ResponseBody(error) => {
                write!(formatter, "Erreur de corps de réponse : {error}")
            }
        }
    }
}
