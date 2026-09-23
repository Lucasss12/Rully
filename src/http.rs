use crate::cli;
use crate::errors::AppError;

fn build_request(cli: &cli::Cli) -> Result<reqwest::RequestBuilder, AppError> {
    let client = reqwest::Client::new();

    let url =
        reqwest::Url::parse(&cli.url).map_err(|error| AppError::InvalidUrl(error.to_string()))?;

    let method = match cli.method {
        cli::Method::Get => reqwest::Method::GET,
        cli::Method::Post => reqwest::Method::POST,
        cli::Method::Put => reqwest::Method::PUT,
        cli::Method::Delete => reqwest::Method::DELETE,
        cli::Method::Patch => reqwest::Method::PATCH,
    };

    let mut queries = Vec::new();

    for query in &cli.queries {
        let (name, value) = query
            .split_once('=')
            .ok_or_else(|| AppError::InvalidQuery("Format attendu : nom=valeur".to_string()))?;
        if url
            .query_pairs()
            .any(|(existing_name, _)| existing_name == name)
        {
            return Err(AppError::InvalidQuery(format!(
                "Paramètre déjà présent dans l'URL : {name}"
            )));
        }
        queries.push((name.to_string(), value.to_string()));
    }

    let mut request = client.request(method, url);

    let mut headers = Vec::new();

    for header in &cli.headers {
        let (name, value) = header
            .split_once(':')
            .ok_or_else(|| AppError::InvalidHeader("Format attendu : Nom: valeur".to_string()))?;
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }

    for (name, value) in &headers {
        request = request.header(name, value);
    }

    request = request.query(&queries);

    let body = match (cli.body.as_deref(), cli.body_file.as_deref()) {
        (Some(_), Some(_)) => return Err(AppError::ConflictingBodyOptions),
        (Some(body), None) => Some(body.to_owned()),
        (None, Some(path)) => Some(std::fs::read_to_string(path).map_err(AppError::FileRead)?),
        (None, None) => None,
    };

    let is_json = headers.iter().any(|(name, value)| {
        let media_type = value.split(';').next().unwrap_or("").trim();
        name.eq_ignore_ascii_case("content-type")
            && media_type.eq_ignore_ascii_case("application/json")
    });

    if let Some(body) = body.as_deref() {
        if is_json {
            serde_json::from_str::<serde_json::Value>(body).map_err(AppError::InvalidJson)?;
        }
        request = request.body(body.to_owned());
    }

    Ok(request)
}

pub(crate) async fn execute(cli: &cli::Cli) -> Result<reqwest::Response, AppError> {
    let request = build_request(cli)?;
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) if error.is_timeout() => return Err(AppError::Timeout),
        Err(error) => return Err(AppError::Network(error.to_string())),
    };

    Ok(response)
}
