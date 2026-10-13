//! End-to-end coverage of the collections workflow against a real server:
//! save a request, read it back from disk, run it, then delete it.

mod support;

use rully_core::errors::AppError;
use rully_core::http;
use rully_core::http_file;
use rully_core::request::Request;
use rully_core::store;
use std::fs;
use std::path::PathBuf;
use support::TestServer;

const OK: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\
                    Content-Length: 5\r\nConnection: close\r\n\r\nhello";

/// Every fixture gets its own directory under the system temp directory and
/// removes it on drop, so the suite stays clean and parallelisable.
struct Project(PathBuf);

impl Project {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rully-e2e-{name}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();

        Self(path)
    }

    fn root(&self) -> PathBuf {
        self.0.join(store::ROOT_DIR)
    }

    fn cwd(&self) -> &PathBuf {
        &self.0
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(store::request_path(&self.root(), relative).unwrap()).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn init(name: &str) -> Project {
    let project = Project::new(name);
    store::init(&project.root()).unwrap();
    project
}

fn parse_request(source: &str) -> Request {
    http_file::parse(source)
        .and_then(Request::try_from)
        .expect("the request should parse")
}

#[tokio::test]
async fn saves_and_runs_a_request() {
    let mut server = TestServer::start(vec![OK.to_vec()]).await;
    let project = init("save-run");
    store::create_collection(&project.root(), "users").unwrap();

    let original = format!("GET {}\nAccept: text/plain", server.url("/greeting"));
    let request = parse_request(&original);

    let text = http_file::to_string(&request).unwrap();
    store::save(&project.root(), "users/greeting", &text).unwrap();

    let from_disk = http_file::read(
        store::request_path(&project.root(), "users/greeting")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .and_then(Request::try_from)
    .unwrap();

    let response = http::execute(&http::default_client().unwrap(), &from_disk)
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "hello");

    let sent = server.next_request().await;
    assert!(sent.contains("accept: text/plain"), "{sent}");
    assert!(sent.contains("GET /greeting "), "{sent}");
}

#[tokio::test]
async fn a_saved_post_sends_its_body_and_headers() {
    let mut server = TestServer::start(vec![OK.to_vec()]).await;
    let project = init("save-post");
    store::create_collection(&project.root(), "users").unwrap();

    let original = format!(
        "POST {}\n\
         Content-Type: application/json\n\
         Authorization: Bearer secret\n\n\
         {{\"name\":\"John\"}}",
        server.url("/users")
    );
    let request = parse_request(&original);
    assert!(request.validate_json_body().is_ok());

    let text = http_file::to_string(&request).unwrap();
    store::save(&project.root(), "users/create", &text).unwrap();

    let from_disk = http_file::read(
        store::request_path(&project.root(), "users/create")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .and_then(Request::try_from)
    .unwrap();

    let response = http::execute(&http::default_client().unwrap(), &from_disk)
        .await
        .unwrap();

    assert_eq!(response.status, 200);

    let sent = server.next_request().await;
    assert!(sent.contains("\"name\":\"John\""), "{sent}");
    assert!(sent.contains("authorization: Bearer secret"), "{sent}");
}

#[tokio::test]
async fn round_trips_every_header_and_the_body_through_the_disk() {
    let project = init("round-trip");
    store::create_collection(&project.root(), "users").unwrap();

    let original = "POST https://api.example.com/users?page=1\n\
                    Accept: application/json\n\
                    Authorization: Bearer abc123\n\
                    X-Trace: 42\n\
                    Content-Type: text/plain\n\n\
                    line one\n\n\
                    line three\n\
                    ";
    let request = parse_request(original);
    let text = http_file::to_string(&request).unwrap();
    store::save(&project.root(), "users/edge", &text).unwrap();

    let reread = http_file::read(
        store::request_path(&project.root(), "users/edge")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .and_then(Request::try_from)
    .unwrap();

    assert_eq!(request.method, reread.method);
    assert_eq!(request.url, reread.url);
    assert_eq!(request.body, reread.body);

    for name in ["accept", "authorization", "x-trace", "content-type"] {
        assert_eq!(request.headers[name], reread.headers[name], "{name}");
    }
}

#[tokio::test]
async fn deletes_a_saved_request() {
    let project = init("delete");
    store::create_collection(&project.root(), "users").unwrap();
    store::save(
        &project.root(),
        "users/greeting",
        "GET https://x.example.com\n",
    )
    .unwrap();

    store::delete(&project.root(), "users/greeting").unwrap();

    assert!(
        store::list_requests(&project.root(), "users")
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        store::delete(&project.root(), "users/greeting"),
        Err(AppError::RequestNotFound(_))
    ));
}

#[tokio::test]
async fn a_new_request_is_immediately_runnable() {
    let server = TestServer::start(vec![OK.to_vec()]).await;
    let project = init("new-run");
    store::create_collection(&project.root(), "users").unwrap();

    store::new_request(&project.root(), "users/skeleton.http").unwrap();

    // The template points at example.com, so rewrite only the URL line and
    // keep everything else exactly as `rully new` wrote it.
    let template = project.read("users/skeleton.http");
    let replaced = template.replace("https://api.example.com", &server.url("/"));
    store::delete(&project.root(), "users/skeleton.http").unwrap();
    store::save(&project.root(), "users/skeleton.http", &replaced).unwrap();

    let request = http_file::read(
        store::request_path(&project.root(), "users/skeleton.http")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .and_then(Request::try_from)
    .unwrap();

    let response = http::execute(&http::default_client().unwrap(), &request)
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert!(
        template.contains("#"),
        "the template should explain the format"
    );
}

#[tokio::test]
async fn finds_the_workspace_from_a_subdirectory() {
    let project = init("subdir");
    let nested = store::collections_dir(&project.root()).join("users");
    fs::create_dir_all(&nested).unwrap();

    let root = store::open(&nested).unwrap();

    assert_eq!(root, project.root());
    assert_eq!(root, store::open(project.cwd()).unwrap());
}

#[tokio::test]
async fn a_nested_collection_round_trips() {
    let project = init("nested");
    store::create_collection(&project.root(), "users").unwrap();
    fs::create_dir_all(store::collections_dir(&project.root()).join("users/admin")).unwrap();

    let text = http_file::to_string(&parse_request("GET https://api.example.com\n")).unwrap();
    store::save(&project.root(), "users/admin/delete", &text).unwrap();

    let reread = http_file::read(
        store::request_path(&project.root(), "users/admin/delete")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .and_then(Request::try_from)
    .unwrap();

    assert_eq!(reread.url.as_str(), "https://api.example.com/");
}
