use std::future::pending;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

pub struct TestServer {
    addr: SocketAddr,
    requests: mpsc::UnboundedReceiver<String>,
}

impl TestServer {
    pub async fn start(responses: Vec<Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (sender, requests) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            for response in responses.into_iter().cycle() {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };

                let mut buffer = vec![0u8; 4096];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let _ = sender.send(String::from_utf8_lossy(&buffer[..read]).into_owned());

                let _ = socket.write_all(&response).await;
                let _ = socket.shutdown().await;
            }
        });

        Self { addr, requests }
    }

    pub async fn start_hanging() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (sender, requests) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = vec![0u8; 4096];
                let _ = socket.read(&mut buffer).await;
                let _ = sender.send(String::new());
                pending::<()>().await;
            }
        });

        Self { addr, requests }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    pub async fn next_request(&mut self) -> String {
        tokio::time::timeout(Duration::from_secs(2), self.requests.recv())
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
    }
}
