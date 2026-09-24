//! Retrieval-provider boundary for task context assembly.
//!
//! The runtime asks a provider for ranked context candidates for a task and
//! stage; the provider returns evidence with provenance and confidence. The
//! runtime keeps authority over budgets, inclusion thresholds, and role
//! filtering. A generic repository-local provider is the default and fallback.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use opencode_types::{
    RetrievalCandidate, RetrievalCandidateKind, RetrievalConfidence, RetrievalRequest,
    RetrievalResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum RetrievalError {
    #[error("retrieval provider unavailable: {0}")]
    Unavailable(String),
    #[error("retrieval provider failed: {0}")]
    Failed(String),
}

/// A source of ranked context candidates for a task and stage.
#[async_trait]
pub trait RetrievalProvider: Send + Sync {
    /// Stable provider name, surfaced in provenance.
    fn name(&self) -> &str;

    /// Return ranked candidates for the request. Providers must not mutate task
    /// state or make inclusion decisions; they only return evidence.
    async fn retrieve(
        &self,
        request: &RetrievalRequest,
    ) -> Result<RetrievalResponse, RetrievalError>;
}

/// Generic repository-local provider.
///
/// It resolves explicit seed files and changed files under the workspace root
/// and returns them as exact, provenance-labeled candidates, plus seed symbols
/// as heuristic candidates. It deliberately does not implement structural
/// graph retrieval; that is a future provider's job.
#[derive(Debug, Default, Clone, Copy)]
pub struct GenericRepositoryProvider;

impl GenericRepositoryProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RetrievalProvider for GenericRepositoryProvider {
    fn name(&self) -> &str {
        "generic"
    }

    async fn retrieve(
        &self,
        request: &RetrievalRequest,
    ) -> Result<RetrievalResponse, RetrievalError> {
        let root = Path::new(&request.workspace_root);
        let mut candidates = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for path in request
            .seed_files
            .iter()
            .chain(request.changed_files.iter())
        {
            let resolved = resolve(root, path);
            if !resolved.exists() {
                continue;
            }
            let key = format!("file:{}", resolved.display());
            if !seen.insert(key) {
                continue;
            }
            let provenance = if request.changed_files.contains(path) {
                "changed file"
            } else {
                "seed file"
            };
            candidates.push(RetrievalCandidate {
                kind: RetrievalCandidateKind::File,
                path: resolved.to_string_lossy().into_owned(),
                symbol: None,
                snippet: None,
                provenance: provenance.to_string(),
                confidence: RetrievalConfidence::Exact,
                score: 1.0,
                estimated_tokens: None,
                origin: None,
                lifecycle: None,
            });
        }

        for symbol in &request.seed_symbols {
            let key = format!("symbol:{symbol}");
            if !seen.insert(key) {
                continue;
            }
            candidates.push(RetrievalCandidate {
                kind: RetrievalCandidateKind::Symbol,
                path: request.workspace_root.clone(),
                symbol: Some(symbol.clone()),
                snippet: None,
                provenance: "seed symbol".to_string(),
                confidence: RetrievalConfidence::Medium,
                score: 0.5,
                estimated_tokens: None,
                origin: None,
                lifecycle: None,
            });
        }

        // Exact repository facts outrank heuristic relationships.
        candidates
            .sort_by_key(|candidate| std::cmp::Reverse(confidence_rank(candidate.confidence)));

        Ok(RetrievalResponse {
            candidates,
            provider: self.name().to_string(),
            plan_signals: Vec::new(),
        })
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

fn resolve(root: &Path, path: &str) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencode_types::{RetrievalRole, SessionTask, TaskStage};

    fn request(workspace_root: &Path) -> RetrievalRequest {
        RetrievalRequest {
            objective: "fix the parser".to_string(),
            stage: TaskStage::Implementing,
            workspace_root: workspace_root.to_string_lossy().into_owned(),
            seed_files: Vec::new(),
            seed_symbols: Vec::new(),
            changed_files: Vec::new(),
            role: RetrievalRole::Implementing,
            token_budget: None,
            task_id: None,
            plan_nodes: Vec::new(),
        }
    }

    #[tokio::test]
    async fn returns_existing_seed_file_with_provenance() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lib.rs"), "fn main() {}").unwrap();

        let provider = GenericRepositoryProvider::new();
        let mut req = request(dir.path());
        req.seed_files.push("lib.rs".to_string());
        req.seed_files.push("missing.rs".to_string());

        let response = provider.retrieve(&req).await.unwrap();

        assert_eq!(response.provider, "generic");
        assert_eq!(response.candidates.len(), 1);
        let candidate = &response.candidates[0];
        assert_eq!(candidate.kind, RetrievalCandidateKind::File);
        assert_eq!(candidate.confidence, RetrievalConfidence::Exact);
        assert_eq!(candidate.provenance, "seed file");
        assert!(candidate.path.ends_with("lib.rs"));
    }

    #[tokio::test]
    async fn dedupes_overlapping_seed_and_changed_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();

        let provider = GenericRepositoryProvider::new();
        let mut req = request(dir.path());
        req.seed_files.push("a.rs".to_string());
        req.changed_files.push("a.rs".to_string());

        let response = provider.retrieve(&req).await.unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(response.candidates[0].provenance, "changed file");
    }

    #[tokio::test]
    async fn seed_symbols_are_heuristic_and_rank_below_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();

        let provider = GenericRepositoryProvider::new();
        let mut req = request(dir.path());
        req.seed_files.push("a.rs".to_string());
        req.seed_symbols.push("parse_config".to_string());

        let response = provider.retrieve(&req).await.unwrap();
        assert_eq!(response.candidates.len(), 2);
        assert_eq!(
            response.candidates[0].confidence,
            RetrievalConfidence::Exact
        );
        assert_eq!(
            response.candidates[1].confidence,
            RetrievalConfidence::Medium
        );
    }

    #[test]
    fn request_from_task_carries_authoritative_fields() {
        let task = SessionTask::new("ship the boundary", vec!["done".into()], "/tmp/ws", vec![]);
        let req = RetrievalRequest::from_task(&task, RetrievalRole::Reviewing);

        assert_eq!(req.objective, "ship the boundary");
        assert_eq!(req.workspace_root, "/tmp/ws");
        assert_eq!(req.stage, TaskStage::Selected);
        assert_eq!(req.role, RetrievalRole::Reviewing);
    }

    #[test]
    fn empty_response_has_no_candidates() {
        let response = RetrievalResponse::empty("generic");
        assert!(response.candidates.is_empty());
        assert_eq!(response.provider, "generic");
    }
}
