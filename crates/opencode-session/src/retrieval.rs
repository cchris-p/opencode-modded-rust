//! Runtime-side retrieval-provider boundary.
//!
//! The runtime derives a retrieval request from authoritative task state and
//! explicit seed files, then asks a provider for ranked candidates. The generic
//! repository-local provider is the default and fallback; providers only return
//! evidence and never mutate task state.

use opencode_retrieval::{GenericRepositoryProvider, RetrievalProvider};
use opencode_types::{RetrievalRequest, RetrievalResponse, RetrievalRole, TaskStage};

use crate::prompt::ModelRef;
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
            task_id: None,
            plan_nodes: Vec::new(),
        },
    }
}

/// Ask the provider selected for the active model for candidates.
///
/// The scopemux provider is scoped to the local Qwen-via-Ollama model; every
/// other model uses the generic repository-local provider. Returns `None` on
/// provider failure so assembly continues with generic behavior; the provider
/// is optional by design.
pub async fn retrieve(
    session: &Session,
    role: RetrievalRole,
    seed_files: Vec<String>,
    model: Option<&ModelRef>,
) -> Option<RetrievalResponse> {
    let request = build_request(session, role, seed_files);

    // Select by the active model: scopemux only for local qwen, generic
    // otherwise. Fall back to the generic provider when scopemux is unavailable.
    let provider = opencode_scopemux::provider_for(
        model.map(|m| m.provider_id.as_str()),
        model.map(|m| m.model_id.as_str()),
    );
    match provider.retrieve(&request).await {
        Ok(response) => Some(response),
        Err(error) => {
            if provider.name() == "generic" {
                tracing::warn!("generic retrieval provider failed: {error}");
                return None;
            }
            tracing::debug!(
                provider = provider.name(),
                "retrieval provider unavailable ({error}); falling back to generic"
            );
            match GenericRepositoryProvider::new().retrieve(&request).await {
                Ok(response) => Some(response),
                Err(error) => {
                    tracing::warn!("generic retrieval provider failed: {error}");
                    None
                }
            }
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
        assert!(
            request.task_id.is_some(),
            "task id should flow into the request"
        );
        assert_eq!(
            request.plan_nodes.len(),
            2,
            "objective and completion criterion should project to plan nodes"
        );
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
            None,
        )
        .await
        .expect("generic provider should succeed");

        assert_eq!(response.provider, "generic");
        assert_eq!(response.candidates.len(), 1);
        assert!(response.candidates[0].path.ends_with("a.rs"));
    }

    #[tokio::test]
    async fn retrieve_uses_generic_for_non_local_models() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn main() {}").unwrap();
        let session = session_with_task(&dir.path().to_string_lossy());
        let model = ModelRef {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-flash".to_string(),
        };

        let response = retrieve(
            &session,
            RetrievalRole::Implementing,
            vec!["a.rs".to_string()],
            Some(&model),
        )
        .await
        .expect("generic provider should succeed");

        assert_eq!(response.provider, "generic");
    }
}
