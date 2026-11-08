//! Integration test for the native remote-Ollama path.
//!
//! The stub server speaks the two endpoints the product probes (`/v1/models`,
//! `/api/tags`) and an OpenAI-compatible chat completion. Nothing in this test
//! invokes the `ollama` binary; the provider only uses HTTP.

use opencode_provider::message::{ChatRequest, Content, Message, Role};
use opencode_provider::ollama::{self, OllamaConfigFile, OllamaEnv, OllamaSource};
use opencode_provider::openai::OpenAIProvider;
use opencode_provider::Provider;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn spawn_stub_server() -> (String, Arc<AtomicBool>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub server");
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();

    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        while !stop_thread.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
                    let mut buffer = Vec::new();
                    let mut chunk = [0u8; 4096];
                    let (path, content_length) = loop {
                        match stream.read(&mut chunk) {
                            Ok(0) => break (String::new(), 0),
                            Ok(n) => {
                                buffer.extend_from_slice(&chunk[..n]);
                                if let Some(header_end) =
                                    buffer.windows(4).position(|window| window == b"\r\n\r\n")
                                {
                                    let head = String::from_utf8_lossy(&buffer[..header_end]);
                                    let path = head
                                        .lines()
                                        .next()
                                        .and_then(|line| line.split_whitespace().nth(1))
                                        .unwrap_or("")
                                        .to_string();
                                    let content_length = head
                                        .lines()
                                        .find_map(|line| {
                                            let (name, value) = line.split_once(':')?;
                                            name.eq_ignore_ascii_case("content-length")
                                                .then(|| value.trim().parse::<usize>().ok())?
                                        })
                                        .unwrap_or(0);
                                    let already = buffer.len() - (header_end + 4);
                                    let mut remaining = content_length.saturating_sub(already);
                                    while remaining > 0 {
                                        match stream.read(&mut chunk) {
                                            Ok(0) => break,
                                            Ok(n) => {
                                                let take = n.min(remaining);
                                                buffer.extend_from_slice(&chunk[..take]);
                                                remaining -= take;
                                            }
                                            Err(_) => break,
                                        }
                                    }
                                    break (path, content_length);
                                }
                            }
                            Err(_) => break (String::new(), 0),
                        }
                    };

                    let (status, body) = match path.as_str() {
                        "/v1/models" => (
                            "200 OK",
                            r#"{"object":"list","data":[{"id":"llama3.2","object":"model"}]}"#,
                        ),
                        "/api/tags" => (
                            "200 OK",
                            r#"{"models":[{"name":"llama3.2:latest","model":"llama3.2:latest"}]}"#,
                        ),
                        "/v1/chat/completions" => (
                            "200 OK",
                            r#"{"id":"chatcmpl-stub","object":"chat.completion","created":0,"model":"llama3.2","choices":[{"index":0,"message":{"role":"assistant","content":"hello from remote stub"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":4,"total_tokens":7}}"#,
                        ),
                        _ => ("404 Not Found", "{}"),
                    };

                    let response = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                    let _ = content_length;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });

    (format!("http://{addr}"), stop, handle)
}

#[tokio::test]
async fn remote_endpoint_resolves_probes_and_completes_without_ollama_binary() {
    let (host, stop, handle) = spawn_stub_server();

    // Resolution uses the config-file source, the lowest explicit precedence
    // tier above OLLAMA_HOST/default.
    let resolution = ollama::resolve_with(
        None,
        &OllamaEnv::default(),
        Path::new("/tmp/ollama.json"),
        Some(&OllamaConfigFile {
            host: Some(host.clone()),
            model: Some("llama3.2".to_string()),
        }),
    );

    assert_eq!(resolution.base_url, format!("{host}/v1"));
    assert_eq!(resolution.model_ref.as_deref(), Some("ollama/llama3.2"));
    assert_eq!(resolution.host_source, OllamaSource::ConfigFile);
    assert_eq!(resolution.model_source, Some(OllamaSource::ConfigFile));

    // Probe succeeds against /v1/models.
    ollama::probe(&resolution, Duration::from_secs(2))
        .await
        .expect("probe should succeed");

    // Both advertised endpoints are reachable over HTTP.
    let client = reqwest::Client::new();
    let models = client
        .get(format!("{host}/v1/models"))
        .send()
        .await
        .expect("GET /v1/models");
    assert!(models.status().is_success());
    let tags = client
        .get(format!("{host}/api/tags"))
        .send()
        .await
        .expect("GET /api/tags");
    assert!(tags.status().is_success());

    // A configured non-default tag is used end-to-end against the stub.
    let provider = OpenAIProvider::openai_compatible(&resolution.base_url, "ollama");
    let response = provider
        .chat(ChatRequest {
            model: resolution.model.clone().unwrap(),
            messages: vec![Message {
                role: Role::User,
                content: Content::Text("hello".to_string()),
                cache_control: None,
                provider_options: None,
            }],
            max_tokens: Some(16),
            temperature: None,
            top_p: None,
            system: None,
            tools: None,
            stream: Some(false),
            provider_options: None,
            variant: None,
        })
        .await
        .expect("chat completion against stub");

    assert_eq!(response.choices.len(), 1);
    match &response.choices[0].message.content {
        Content::Text(text) => assert_eq!(text, "hello from remote stub"),
        other => panic!("unexpected content: {other:?}"),
    }

    stop.store(true, Ordering::Relaxed);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !handle.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
}

#[tokio::test]
async fn probe_reports_url_and_reason_when_endpoint_is_unreachable() {
    let resolution = ollama::resolve_with(
        None,
        &OllamaEnv {
            remote_host: Some("http://127.0.0.1:1".to_string()),
            remote_model: Some("llama3.2".to_string()),
            ollama_host: None,
        },
        Path::new("/tmp/ollama.json"),
        None,
    );

    let reason = ollama::probe(&resolution, Duration::from_millis(300))
        .await
        .expect_err("unreachable endpoint must fail");
    let message = resolution.connection_failure_message(&reason);
    assert!(message.contains("http://127.0.0.1:1/v1"));
    assert!(message.contains("/tmp/ollama.json"));
    assert!(message.contains("reason:"));
}
