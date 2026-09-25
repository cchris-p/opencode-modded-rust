use serde::{Deserialize, Serialize};

use crate::plan::{
    project_task_to_plan_nodes, PlanLifecycle, PlanNodeDraft, PlanReconciliationSignal,
};
use crate::task::{SessionTask, TaskStage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalRole {
    Implementing,
    Reviewing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalCandidateKind {
    File,
    Symbol,
    Snippet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalConfidence {
    Exact,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalOrigin {
    Parsed,
    Planned,
}

impl RetrievalOrigin {
    pub fn from_ffi(value: i32) -> Self {
        if value == 1 {
            Self::Planned
        } else {
            Self::Parsed
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalCandidate {
    pub kind: RetrievalCandidateKind,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    pub provenance: String,
    pub confidence: RetrievalConfidence,
    #[serde(default)]
    pub score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_tokens: Option<usize>,
    /// Whether the candidate is parsed fact or a projected plan node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<RetrievalOrigin>,
    /// Plan lifecycle for planned candidates; parsed facts use `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<PlanLifecycle>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalRequest {
    pub objective: String,
    pub stage: TaskStage,
    pub workspace_root: String,
    #[serde(default)]
    pub seed_files: Vec<String>,
    #[serde(default)]
    pub seed_symbols: Vec<String>,
    #[serde(default)]
    pub changed_files: Vec<String>,
    pub role: RetrievalRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Projection-only plan nodes derived from the authoritative task record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_nodes: Vec<PlanNodeDraft>,
}

impl RetrievalRequest {
    pub fn from_task(task: &SessionTask, role: RetrievalRole) -> Self {
        Self {
            objective: task.objective.clone(),
            stage: task.stage.clone(),
            workspace_root: task.workspace_target.clone(),
            seed_files: Vec::new(),
            seed_symbols: Vec::new(),
            changed_files: task.artifacts.clone(),
            role,
            token_budget: None,
            task_id: Some(task.task_id.clone()),
            plan_nodes: project_task_to_plan_nodes(task),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalResponse {
    pub candidates: Vec<RetrievalCandidate>,
    pub provider: String,
    /// Reconciliation evidence for projected plan nodes; the runtime decides
    /// whether to act on it. Never advances task stage or completion.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_signals: Vec<PlanReconciliationSignal>,
}

impl RetrievalResponse {
    pub fn empty(provider: impl Into<String>) -> Self {
        Self {
            candidates: Vec::new(),
            provider: provider.into(),
            plan_signals: Vec::new(),
        }
    }
}
