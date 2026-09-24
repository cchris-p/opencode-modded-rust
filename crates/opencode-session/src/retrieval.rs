//! Runtime-side retrieval-provider boundary.
//!
//! The runtime derives a retrieval request from authoritative task state and
//! explicit seed files, then asks a provider for ranked candidates. The generic
//! repository-local provider is the default and fallback; providers only return
//! evidence and never mutate task state.

use opencode_retrieval::{GenericRepositoryProvider, RetrievalProvider};
use opencode_types::{RetrievalRequest, RetrievalResponse, RetrievalRole, TaskStage};

use crate::session::Session;

/// Build the retrieval request for a session from authoritative task state.
pub fn build_request(
    session: &Session,
    role: RetrievalRole,
    seed_files: Vec<String>,
) -> RetrievalRequest {
    match &session.task {
        Some(task) => {
            let mut request = RetrievalRequest::from_task(task, role);
            request.seed_files = seed_files;
            request
        }
        None => RetrievalRequest {
            objective: String::new(),
            stage: TaskStage::Selected,
            workspace_root: session.directory.clone(),
            seed_files,
            seed_symbols: Vec::new(),
            changed_files: Vec::new(),
            role,
            token_budget: None,
        },
    }
}

/// Ask the default provider for candidates.
///
/// Returns `None` on provider failure so assembly continues with generic
/// behavior; the provider is optional by design.
pub async fn retrieve(
    session: &Session,
    role: RetrievalRole,
    seed_files: Vec<String>,
) -> Option<RetrievalResponse> {
    let request = build_request(session, role, seed_files);
    match GenericRepositoryProvider::new().retrieve(&request).await {
        Ok(response) => Some(response),
        Err(error) => {
            tracing::warn!("generic retrieval provider failed: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencode_types::SessionTask;

    fn session_with_task(root: &str) -> Session {
        let mut session = Session::new("project", root);
        session.set_task(SessionTask::new(
            "objective",
            vec!["criteria".to_string()],
            root,
            Vec::new(),
        ));
        session
    }

    #[test]
    fn build_request_uses_task_fields() {
        let session = session_with_task("/tmp/ws");
        let request = build_request(&session, RetrievalRole::Reviewing, vec!["a.rs".to_string()]);

        assert_eq!(request.objective, "objective");
        assert_eq!(request.stage, TaskStage::Selected);
        assert_eq!(request.workspace_root, "/tmp/ws");
        assert_eq!(request.role, RetrievalRole::Reviewing);
        assert_eq!(request.seed_files, vec!["a.rs".to_string()]);
    }

    #[tokio::test]
    async fn retrieve_returns_candidates_for_existing_seed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn main() {}").unwrap();
        let session = session_with_task(&dir.path().to_string_lossy());

        let response = retrieve(
            &session,
            RetrievalRole::Implementing,
            vec!["a.rs".to_string()],
        )
        .await
        .expect("generic provider should succeed");

        assert_eq!(response.provider, "generic");
        assert_eq!(response.candidates.len(), 1);
        assert!(response.candidates[0].path.ends_with("a.rs"));
    }
}
