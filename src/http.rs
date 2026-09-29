use crate::errors::AppError;
use crate::request::Request;
use crate::response::Response;
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, HeaderMap, HeaderValue, USER_AGENT};
use std::time::{Duration, Instant};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip"));
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));

    headers
}

pub fn default_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .default_headers(default_headers())
        .build()
        .map_err(|error| AppError::ClientBuild(error.to_string()))
}

fn build_request(client: &reqwest::Client, request: &Request) -> reqwest::RequestBuilder {
    let mut builder = client.request(request.method.clone(), request.url.clone());

    builder = builder.headers(request.headers.clone());

    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }

    builder
}

pub async fn execute(client: &reqwest::Client, request: &Request) -> Result<Response, AppError> {
    request.validate_json_body()?;

    let builder = build_request(client, request);
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
