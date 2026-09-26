//! Runtime-side retrieval-provider boundary.
//!
//! The runtime derives a retrieval request from authoritative task state and
//! explicit seed files, then asks a provider for ranked candidates. The generic
//! repository-local provider is the default and fallback; providers only return
//! evidence and never mutate task state.
//!
//! The request's map role and representation slice are derived from the task
//! stage (see [`opencode_types::stage_retrieval`]), so each stage asks the map
//! for the representation it needs. The runtime keeps authority over inclusion
//! thresholds and token budgets.

use opencode_retrieval::{GenericRepositoryProvider, RetrievalProvider};
use opencode_types::{
    stage_retrieval, RetrievalConfidence, RetrievalRepresentation, RetrievalRequest,
    RetrievalResponse, TaskStage,
};

use crate::prompt::ModelRef;
use crate::session::Session;

/// Build the retrieval request for a session from authoritative task state.
///
/// The role and representation are derived from the task stage; a session with
/// no task uses the `Selected` stage mapping.
pub fn build_request(session: &Session, seed_files: Vec<String>) -> RetrievalRequest {
    match &session.task {
        Some(task) => {
            let mut request = RetrievalRequest::from_task(task);
            request.seed_files = seed_files;
            request
        }
        None => {
            let stage = stage_retrieval(&TaskStage::Selected);
            RetrievalRequest {
                objective: String::new(),
                stage: TaskStage::Selected,
                workspace_root: session.directory.clone(),
                seed_files,
                seed_symbols: Vec::new(),
                changed_files: Vec::new(),
                role: stage.role,
                representation: stage.representation,
                token_budget: None,
                task_id: None,
                reopen_reason: None,
                plan_nodes: Vec::new(),
            }
        }
    }
}

fn confidence_rank(confidence: RetrievalConfidence) -> u8 {
    match confidence {
        RetrievalConfidence::Exact => 4,
        RetrievalConfidence::High => 3,
        RetrievalConfidence::Medium => 2,
        RetrievalConfidence::Low => 1,
    }
}

/// Lowest confidence a representation admits.
///
/// Review keeps only exact and high-confidence evidence so heuristic links
/// cannot enter review context; implementation and the locate/repair slices may
/// keep heuristic links as enrichment (`invariants/retrieval.md`).
pub fn min_confidence(representation: RetrievalRepresentation) -> RetrievalConfidence {
    match representation {
        RetrievalRepresentation::Review => RetrievalConfidence::High,
        _ => RetrievalConfidence::Low,
    }
}

/// Apply the runtime-side inclusion threshold to a provider response.
///
/// Providers return evidence; the runtime decides what is included. Plan
/// reconciliation signals pass through untouched.
pub fn apply_inclusion_threshold(mut response: RetrievalResponse) -> RetrievalResponse {
    let minimum = confidence_rank(min_confidence(response.representation));
    response
        .candidates
        .retain(|candidate| confidence_rank(candidate.confidence) >= minimum);
    response
}

/// Ask the provider selected for the active model for candidates.
///
/// The scopemux provider is scoped to the local Qwen-via-Ollama model; every
/// other model uses the generic repository-local provider. Returns `None` on
/// provider failure so assembly continues with generic behavior; the provider
/// is optional by design.
pub async fn retrieve(
    session: &Session,
    seed_files: Vec<String>,
    model: Option<&ModelRef>,
) -> Option<RetrievalResponse> {
    let request = build_request(session, seed_files);

    // Select by the active model: scopemux only for local qwen, generic
    // otherwise. Fall back to the generic provider when scopemux is unavailable.
    let provider = opencode_scopemux::provider_for(
        model.map(|m| m.provider_id.as_str()),
        model.map(|m| m.model_id.as_str()),
    );
    let response = match provider.retrieve(&request).await {
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
    };

    response.map(apply_inclusion_threshold)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencode_types::{
        RetrievalCandidate, RetrievalCandidateKind, RetrievalOrigin, SessionTask, TaskStage,
    };

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

    fn candidate(confidence: RetrievalConfidence, path: &str) -> RetrievalCandidate {
        RetrievalCandidate {
            kind: RetrievalCandidateKind::Symbol,
            path: path.to_string(),
            symbol: None,
            snippet: None,
            provenance: "test".to_string(),
            confidence,
            score: 0.0,
            estimated_tokens: None,
            origin: Some(RetrievalOrigin::Parsed),
            lifecycle: None,
        }
    }

    #[test]
    fn build_request_derives_role_and_representation_from_stage() {
        let mut session = session_with_task("/tmp/ws");
        let task = session.task.as_mut().unwrap();
        task.stage = TaskStage::Reviewing;

        let request = build_request(&session, vec!["a.rs".to_string()]);

        assert_eq!(request.objective, "objective");
        assert_eq!(request.stage, TaskStage::Reviewing);
        assert_eq!(request.workspace_root, "/tmp/ws");
        assert_eq!(request.role, opencode_types::MapRole::Project);
        assert_eq!(request.representation, RetrievalRepresentation::Review);
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

    #[test]
    fn review_threshold_drops_heuristic_candidates() {
        let response = RetrievalResponse {
            candidates: vec![
                candidate(RetrievalConfidence::Exact, "exact.rs"),
                candidate(RetrievalConfidence::High, "high.rs"),
                candidate(RetrievalConfidence::Medium, "medium.rs"),
                candidate(RetrievalConfidence::Low, "low.rs"),
            ],
            provider: "scopemux".to_string(),
            representation: RetrievalRepresentation::Review,
            plan_signals: Vec::new(),
        };

        let filtered = apply_inclusion_threshold(response);

        let paths: Vec<&str> = filtered
            .candidates
            .iter()
            .map(|c| c.path.as_str())
            .collect();
        assert_eq!(paths, vec!["exact.rs", "high.rs"]);
    }

    #[test]
    fn implementation_threshold_keeps_heuristic_enrichment() {
        let response = RetrievalResponse {
            candidates: vec![candidate(RetrievalConfidence::Low, "low.rs")],
            provider: "scopemux".to_string(),
            representation: RetrievalRepresentation::Delta,
            plan_signals: Vec::new(),
        };

        let filtered = apply_inclusion_threshold(response);

        assert_eq!(filtered.candidates.len(), 1);
    }

    #[tokio::test]
    async fn retrieve_returns_candidates_for_existing_seed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn main() {}").unwrap();
        let session = session_with_task(&dir.path().to_string_lossy());

        let response = retrieve(&session, vec!["a.rs".to_string()], None)
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

        let response = retrieve(&session, vec!["a.rs".to_string()], Some(&model))
            .await
            .expect("generic provider should succeed");

        assert_eq!(response.provider, "generic");
    }
}
