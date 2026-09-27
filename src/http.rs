use crate::errors::AppError;
use crate::request::Request;
use crate::response::Response;
use std::time::Instant;

fn build_request(request: &Request) -> reqwest::RequestBuilder {
    let client = reqwest::Client::new();

    let mut builder = client.request(request.method.clone(), request.url.clone());

    builder = builder.headers(request.headers.clone());

    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }

    builder
}

pub async fn execute(request: &Request) -> Result<Response, AppError> {
    request.validate_json_body()?;

    let builder = build_request(request);
    let start = Instant::now();

    let raw = match builder.send().await {
        Ok(response) => response,
        Err(error) if error.is_timeout() => return Err(AppError::Timeout),
        Err(error) => return Err(AppError::Network(error.to_string())),
    };

    let status = raw.status();
    let headers = raw.headers().clone();
    let body = raw
        .bytes()
        .await
        .map_err(|error| AppError::ResponseBody(error.to_string()))?;

    Ok(Response::new(status, headers, body, start.elapsed()))
}
