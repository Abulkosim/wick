//! A loopback release server for the updater install tests.

use serde_json::json;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub struct ReleaseServer {
    address: SocketAddr,
    stopped: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl ReleaseServer {
    /// Serves `artifact` as `file` for each updater `platforms` key, plus a tampered copy
    /// (`/tampered.json`) and a manifest whose version doesn't match the signature
    /// (`/mismatch.json`).
    pub fn start(
        version: &str,
        signature: &str,
        artifact: &[u8],
        file: &str,
        platforms: &[&str],
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let manifest = |version: &str, file: &str| {
            let entry = json!({"url": format!("http://{address}/{file}"), "signature": signature});
            let platforms: serde_json::Map<String, serde_json::Value> = platforms
                .iter()
                .map(|platform| (platform.to_string(), entry.clone()))
                .collect();
            serde_json::to_vec(&json!({ "version": version, "platforms": platforms })).unwrap()
        };
        let mut tampered = artifact.to_vec();
        *tampered.last_mut().expect("Release artifact is empty") ^= 1;
        let responses: HashMap<String, Vec<u8>> = HashMap::from([
            ("/latest.json".to_string(), manifest(version, file)),
            ("/tampered.json".to_string(), manifest(version, "tampered")),
            ("/mismatch.json".to_string(), manifest("999.0.0", file)),
            (format!("/{file}"), artifact.to_vec()),
            ("/tampered".to_string(), tampered),
        ]);
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stopped = stopped.clone();
        let worker = thread::spawn(move || {
            for connection in listener.incoming() {
                if worker_stopped.load(Ordering::SeqCst) {
                    break;
                }
                let mut stream = connection.unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 1024];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let length = stream.read(&mut chunk).unwrap();
                    assert!(length > 0 && request.len() + length <= 16_384);
                    request.extend_from_slice(&chunk[..length]);
                }
                let request = String::from_utf8(request).unwrap();
                let mut words = request.split_whitespace();
                assert_eq!(words.next(), Some("GET"));
                let path = words.next().unwrap();
                let body = responses.get(path).expect("Unexpected loopback request");
                let content_type = if path.ends_with(".json") {
                    "application/json"
                } else {
                    "application/octet-stream"
                };
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(body).unwrap();
            }
        });
        Self {
            address,
            stopped,
            worker: Some(worker),
        }
    }

    pub fn url(&self, path: &str) -> tauri::Url {
        format!("http://{}{path}", self.address).parse().unwrap()
    }
}

impl Drop for ReleaseServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            let result = worker.join();
            if !thread::panicking() {
                result.unwrap();
            }
        }
    }
}
