use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread;

use clawbrowser::config::FetchConfig;
use clawbrowser::engine::Page;

static SERVER_PORT: OnceLock<u16> = OnceLock::new();

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn start_server() -> u16 {
    *SERVER_PORT.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind to random port");
        let port = listener.local_addr().unwrap().port();
        let root = tests_dir();

        thread::spawn(move || {
            let server =
                tiny_http::Server::from_listener(listener, None::<tiny_http::SslConfig>)
                    .expect("create tiny_http server");

            loop {
                let request = match server.recv() {
                    Ok(r) => r,
                    Err(_) => break,
                };

                let url_path = request.url().to_string();
                let url_path = url_path.trim_start_matches('/');
                let file_path = root.join(url_path);

                if file_path.is_file() {
                    let data = std::fs::read(&file_path).unwrap_or_default();
                    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
                        Some("html") => "text/html; charset=utf-8",
                        Some("js") => "text/javascript; charset=utf-8",
                        Some("json") => "application/json; charset=utf-8",
                        Some("css") => "text/css; charset=utf-8",
                        _ => "application/octet-stream",
                    };
                    let header =
                        tiny_http::Header::from_bytes(b"Content-Type", content_type.as_bytes())
                            .unwrap();
                    let response = tiny_http::Response::from_data(data).with_header(header);
                    let _ = request.respond(response);
                } else if url_path == "__echo" {
                    let body = "ok";
                    let header =
                        tiny_http::Header::from_bytes(b"Content-Type", b"text/plain").unwrap();
                    let response = tiny_http::Response::from_string(body).with_header(header);
                    let _ = request.respond(response);
                } else if url_path == "__json" {
                    let body = r#"{"status":"ok","value":42}"#;
                    let header =
                        tiny_http::Header::from_bytes(b"Content-Type", b"application/json")
                            .unwrap();
                    let response = tiny_http::Response::from_string(body).with_header(header);
                    let _ = request.respond(response);
                } else {
                    let response = tiny_http::Response::from_string("Not Found")
                        .with_status_code(tiny_http::StatusCode(404));
                    let _ = request.respond(response);
                }
            }
        });

        port
    })
}

async fn run_html_tests(subdir: &str) {
    let port = start_server();
    let dir = tests_dir().join(subdir);

    assert!(dir.is_dir(), "Test directory not found: {}", dir.display());

    let mut html_files: Vec<PathBuf> = Vec::new();
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("html") {
            html_files.push(path.to_path_buf());
        }
    }

    html_files.sort();
    assert!(
        !html_files.is_empty(),
        "No .html test files found in {}",
        dir.display()
    );

    let config = FetchConfig {
        no_js: false,
        timeout_secs: 15,
        wait_ms: 2000,
        ..Default::default()
    };

    let mut passed = 0u32;
    let mut failures: Vec<String> = Vec::new();

    for html_file in &html_files {
        let relative = html_file
            .strip_prefix(&tests_dir())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");

        let url = format!("http://127.0.0.1:{}/{}", port, relative);

        let result = Page::navigate(&url, &config).await;

        match result {
            Ok(page) => {
                let title = page.title().unwrap_or_default();
                if title == "PASS" {
                    passed += 1;
                } else {
                    failures.push(format!("{}: {}", relative, title));
                }
            }
            Err(e) => {
                failures.push(format!("{}: navigate error: {}", relative, e));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{} passed, {} failed:\n  {}\n",
            passed,
            failures.len(),
            failures.join("\n  ")
        );
    }
}

// --- P0: Core DOM ---

#[tokio::test(flavor = "multi_thread")]
async fn dom_node() {
    run_html_tests("dom/node").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn dom_element() {
    run_html_tests("dom/element").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn dom_document() {
    run_html_tests("dom/document").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn dom_parsing() {
    run_html_tests("dom/parsing").await;
}

// --- P0: URL ---

#[tokio::test(flavor = "multi_thread")]
async fn url_tests() {
    run_html_tests("url").await;
}

// --- P0: Fetch ---

#[tokio::test(flavor = "multi_thread")]
async fn fetch_tests() {
    run_html_tests("fetch").await;
}

// --- P0: Encoding ---

#[tokio::test(flavor = "multi_thread")]
async fn encoding_tests() {
    run_html_tests("encoding").await;
}

// --- P1: Timers ---

#[tokio::test(flavor = "multi_thread")]
async fn timer_tests() {
    run_html_tests("timers").await;
}

// --- P1: Window ---

#[tokio::test(flavor = "multi_thread")]
async fn window_tests() {
    run_html_tests("window").await;
}

// --- P1: Storage ---

#[tokio::test(flavor = "multi_thread")]
async fn storage_tests() {
    run_html_tests("storage").await;
}

// --- P2: Events ---

#[tokio::test(flavor = "multi_thread")]
async fn event_tests() {
    run_html_tests("events").await;
}
