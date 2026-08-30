use std::sync::{Mutex, MutexGuard};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use once_cell::sync::Lazy;
use opencode_server::{routes, ServerState};
use serde::Deserialize;
use tower::ServiceExt;
use uuid::Uuid;

static CURRENT_DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Deserialize)]
struct SkillSummary {
    name: String,
    description: String,
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
    let skills: Vec<SkillSummary> =
        serde_json::from_slice(&body).expect("response should decode");

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "reviewer");
    assert_eq!(skills[0].description, "Review code changes");

    std::fs::remove_dir_all(&root).expect("temp root should be cleaned up");
}

fn lock_current_dir() -> MutexGuard<'static, ()> {
    CURRENT_DIR_LOCK.lock().expect("current dir lock should work")
}
