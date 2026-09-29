mod support;

use reqwest::Method;
use reqwest::StatusCode;
use reqwest::header::{AUTHORIZATION, HeaderValue};
use rully_core::errors::AppError;
use rully_core::http;
use rully_core::request::Request;
use std::time::Duration;
use support::TestServer;
use tokio::net::TcpListener;

const OK: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\
                    X-Custom: valeur\r\nContent-Length: 5\r\n\
                    Connection: close\r\n\r\nhello";

const NOT_FOUND: &[u8] = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\
                         Connection: close\r\n\r\n";

const SERVER_ERROR: &[u8] = b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\
                            Connection: close\r\n\r\n";

const REDIRECT: &[u8] = b"HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\n\
                         Connection: close\r\n\r\n";

const JSON: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\n\
                     Content-Length: 15\r\nConnection: close\r\n\r\n{\"name\":\"John\"}";

const TRUNCATED: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\ncourt";

/// `{"id":1,"name":"John"}` compressed with gzip, as a server would send it.
const GZIP_BODY: &[u8] = &[
    0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xab, 0x56, 0xca, 0x4c, 0x51, 0xb2,
    0x32, 0xd4, 0x51, 0xca, 0x4b, 0xcc, 0x4d, 0x55, 0xb2, 0x52, 0xf2, 0xca, 0xcf, 0xc8, 0x53, 0xaa,
    0x05, 0x00, 0x31, 0xa3, 0x5a, 0x83, 0x16, 0x00, 0x00, 0x00,
];

fn gzip_response() -> Vec<u8> {
    let mut response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                        Content-Encoding: gzip\r\n"
        .to_vec();

    response.extend_from_slice(format!("Content-Length: {}\r\n", GZIP_BODY.len()).as_bytes());
    response.extend_from_slice(b"Connection: close\r\n\r\n");
    response.extend_from_slice(GZIP_BODY);

    response
}

#[tokio::test]
async fn returns_status_headers_and_body() {
    let server = TestServer::start(vec![OK.to_vec()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.headers["x-custom"], "valeur");
    assert_eq!(response.body, "hello");
    assert_eq!(response.size, 5);
}

#[tokio::test]
async fn measures_a_duration() {
    let server = TestServer::start(vec![OK.to_vec()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert!(response.duration_ms() > 0.0);
}

#[tokio::test]
async fn sends_request_headers() {
    let mut server = TestServer::start(vec![OK.to_vec()]).await;

    let mut request = Request::new(Method::GET, &server.url("/")).unwrap();
    request
        .headers
        .insert(AUTHORIZATION, HeaderValue::from_static("Bearer token"));

    http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    let received = server.next_request().await;

    assert!(
        received
            .to_lowercase()
            .contains("authorization: bearer token")
    );
}

#[tokio::test]
async fn sends_request_body() {
    let mut server = TestServer::start(vec![OK.to_vec()]).await;

    let mut request = Request::new(Method::POST, &server.url("/")).unwrap();
    request.body = Some(r#"{"name":"John"}"#.to_string());

    http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    let received = server.next_request().await;

    assert!(received.contains(r#"{"name":"John"}"#));
}

#[tokio::test]
async fn handles_not_found() {
    let server = TestServer::start(vec![NOT_FOUND.to_vec()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn handles_server_error() {
    let server = TestServer::start(vec![SERVER_ERROR.to_vec()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn follows_redirect() {
    let server = TestServer::start(vec![REDIRECT.to_vec(), OK.to_vec()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, "hello");
}

#[tokio::test]
async fn strips_charset_from_content_type() {
    let server = TestServer::start(vec![JSON.to_vec()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert_eq!(response.content_type(), Some("application/json"));
    assert_eq!(response.body, r#"{"name":"John"}"#);
}

#[tokio::test]
async fn reports_network_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let request = Request::new(Method::GET, &format!("http://{addr}/")).unwrap();

    let result = http::execute(&http::default_client().unwrap(), &request).await;

    assert!(matches!(result, Err(AppError::Network(_))));
}

#[tokio::test]
async fn reports_timeout() {
    let server = TestServer::start_hanging().await;

    let client = reqwest::Client::builder()
        .read_timeout(Duration::from_millis(150))
        .build()
        .unwrap();

    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let result = tokio::time::timeout(Duration::from_secs(5), http::execute(&client, &request))
        .await
        .expect("la requête aurait dû expirer");

    assert!(matches!(result, Err(AppError::Timeout)));
}

#[tokio::test]
async fn reports_response_body_error() {
    let server = TestServer::start(vec![TRUNCATED.to_vec()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let result = http::execute(&http::default_client().unwrap(), &request).await;

    assert!(matches!(result, Err(AppError::ResponseBody(_))));
}

#[tokio::test]
async fn decompresses_a_gzipped_response() {
    let server = TestServer::start(vec![gzip_response()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert_eq!(response.body, r#"{"id":1,"name":"John"}"#);
    assert_eq!(response.size, 22);
}

#[tokio::test]
async fn drops_the_content_encoding_headers_once_decompressed() {
    let server = TestServer::start(vec![gzip_response()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert!(response.headers.get("content-encoding").is_none());
    assert!(response.headers.get("content-length").is_none());
}

#[tokio::test]
async fn asks_the_server_for_a_gzipped_response() {
    let mut server = TestServer::start(vec![OK.to_vec()]).await;
    let request = Request::new(Method::GET, &server.url("/")).unwrap();

    http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    let received = server.next_request().await.to_lowercase();

    assert!(received.contains("accept-encoding: gzip"), "{received}");
}
