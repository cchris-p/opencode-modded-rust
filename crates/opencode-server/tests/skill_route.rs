use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use futures::stream;
use once_cell::sync::Lazy;
use opencode_provider::{
    ChatRequest, ChatResponse, ModelInfo, Provider, ProviderError, StreamEvent, StreamResult,
};
use opencode_server::{routes, ServerState};
use serde::Deserialize;
use serde_json::json;
use tokio::time::{timeout, Duration};
use tower::ServiceExt;
use uuid::Uuid;

static CURRENT_DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Deserialize)]
struct SkillSummary {
    name: String,
    description: String,
}

struct TestProvider {
    model: ModelInfo,
}

#[async_trait]
impl Provider for TestProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn name(&self) -> &str {
        "Mock"
    }

    fn models(&self) -> Vec<ModelInfo> {
        vec![self.model.clone()]
    }

    fn get_model(&self, id: &str) -> Option<&ModelInfo> {
        (id == self.model.id).then_some(&self.model)
    }

    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Err(ProviderError::InvalidRequest(
            "non-streaming chat is not used by this test".to_string(),
        ))
    }

    async fn chat_stream(&self, _request: ChatRequest) -> Result<StreamResult, ProviderError> {
        Ok(Box::pin(stream::iter(vec![
            Ok(StreamEvent::Start),
            Ok(StreamEvent::TextDelta("OK".to_string())),
            Ok(StreamEvent::FinishStep {
                finish_reason: Some("stop".to_string()),
                usage: Default::default(),
                provider_metadata: None,
            }),
            Ok(StreamEvent::Done),
        ])))
    }
}

#[tokio::test]
async fn skill_route_returns_discovered_names_and_descriptions() {
    let _guard = lock_current_dir();
    let original_dir = std::env::current_dir().expect("current dir should resolve");

    let root = std::env::temp_dir().join(format!("opencode-skill-route-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("temp root should be created");
    std::fs::write(root.join(".git"), "gitdir").expect("git marker should write");

    let skill_path = root.join(".opencode/skills/reviewer/SKILL.md");
    std::fs::create_dir_all(skill_path.parent().expect("skill parent should exist"))
        .expect("skill directory should be created");
    std::fs::write(
        &skill_path,
        r#"---
name: reviewer
description: Review code changes
---

# Reviewer
"#,
    )
    .expect("skill file should write");

    std::env::set_current_dir(&root).expect("current dir should switch to temp root");

    let app = routes::router().with_state(std::sync::Arc::new(ServerState::new()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/skill")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");

    std::env::set_current_dir(&original_dir).expect("current dir should restore");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    let skills: Vec<SkillSummary> = serde_json::from_slice(&body).expect("response should decode");

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "reviewer");
    assert_eq!(skills[0].description, "Review code changes");

    std::fs::remove_dir_all(&root).expect("temp root should be cleaned up");
}

fn lock_current_dir() -> MutexGuard<'static, ()> {
    CURRENT_DIR_LOCK
        .lock()
        .expect("current dir lock should work")
}

#[tokio::test]
async fn prompt_route_emits_idle_after_stream_completion() {
    let state = Arc::new(ServerState::new());
    {
        let mut providers = state.providers.write().expect("provider lock should work");
        providers.register(TestProvider {
            model: ModelInfo {
                id: "test-model".to_string(),
                name: "Test Model".to_string(),
                provider: "mock".to_string(),
                context_window: 8192,
                max_output_tokens: 1024,
                supports_vision: false,
                supports_tools: false,
                cost_per_million_input: 0.0,
                cost_per_million_output: 0.0,
            },
        });
    }

    let session_id = {
        let mut sessions = state.sessions.lock().await;
        sessions.create("default", ".").id
    };
    let mut events = state.event_bus.subscribe();
    let app = routes::router().with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/session/{session_id}/prompt"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "message": "reply with exactly OK",
                        "model": "mock/test-model"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::OK);

    let saw_idle = timeout(Duration::from_millis(500), async {
        loop {
            let event = events.recv().await.expect("event bus should remain open");
            let value: serde_json::Value = serde_json::from_str(&event).expect("event is json");
            if value.get("type").and_then(|v| v.as_str()) != Some("session.status") {
                continue;
            }
            if value.get("sessionID").and_then(|v| v.as_str()) != Some(session_id.as_str()) {
                continue;
            }
            if value
                .get("status")
                .and_then(|status| status.get("type"))
                .and_then(|v| v.as_str())
                == Some("idle")
            {
                break true;
            }
        }
    })
    .await
    .unwrap_or(false);

    assert!(saw_idle, "prompt route did not emit idle after completion");
}
