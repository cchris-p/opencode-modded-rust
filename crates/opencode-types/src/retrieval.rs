use serde::{Deserialize, Serialize};

use crate::plan::{
    project_task_to_plan_nodes, PlanLifecycle, PlanNodeDraft, PlanReconciliationSignal,
};
use crate::task::{SessionTask, TaskStage};

/// Map role a runtime stage plays when it asks the map (`SCOPE-004`).
///
/// Mirrors the role column of `wiki/scopemux-map-integration.md`. The runtime
/// keeps authority over budgets and inclusion thresholds; the role only tells a
/// provider which representation to return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapRole {
    /// Which nodes matter for this task and stage (seed nodes, anchors).
    Locate,
    /// Cheapest faithful representation of those nodes at a token budget.
    Slice,
    /// Delta between current state and target state.
    Project,
    /// Keep the map correct after changes; no task authority.
    Reconcile,
}

/// Representation slice a stage requests from the map (`SCOPE-004`).
///
/// A provider that cannot build a map returns only the generic facts it has; the
/// runtime still completes. `Review` is a composed slice: delta plus duplicate
/// and observability signals at a stricter confidence threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalRepresentation {
    /// Tiered search over parsed and planned blocks.
    Search,
    /// Delta between current (parsed) and target (planned) state.
    Delta,
    /// Seed nodes and anchors resolved for a task and stage.
    Anchors,
    /// Observability blocks attached to anchored symbols.
    Observability,
    /// Duplicate/refactor-opportunity clusters.
    Duplicates,
    /// Impact of changed files: their blocks plus anchored plans.
    Impact,
    /// Composed review slice: delta, duplicates, and observability.
    Review,
    /// Reconcile plan nodes from observed state; evidence only.
    Reconcile,
}

/// The role and representation a runtime stage maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageRetrieval {
    pub role: MapRole,
    pub representation: RetrievalRepresentation,
}

/// Map a runtime stage to a map role and representation slice.
///
/// This is the `SCOPE-004` stage table. The mapping is a projection of the task
/// stage; it never advances the stage.
pub fn stage_retrieval(stage: &TaskStage) -> StageRetrieval {
    let (role, representation) = match stage {
        TaskStage::Selected => (MapRole::Locate, RetrievalRepresentation::Search),
        TaskStage::ContextPrepared => (MapRole::Slice, RetrievalRepresentation::Delta),
        TaskStage::Implementing => (MapRole::Project, RetrievalRepresentation::Delta),
        TaskStage::Verifying => (MapRole::Locate, RetrievalRepresentation::Anchors),
        TaskStage::Reviewing => (MapRole::Project, RetrievalRepresentation::Review),
        TaskStage::Repairing => (MapRole::Project, RetrievalRepresentation::Impact),
        TaskStage::Completed => (MapRole::Reconcile, RetrievalRepresentation::Reconcile),
    };
    StageRetrieval {
        role,
        representation,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalCandidateKind {
    File,
    Symbol,
    Snippet,
    /// An observability point (log/metric/assertion/invariant) from the map.
    Observability,
    /// A duplicate/refactor-opportunity cluster from the map.
    RefactorOpportunity,
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
    pub role: MapRole,
    /// Representation slice the stage asks the map for.
    #[serde(default = "default_representation")]
    pub representation: RetrievalRepresentation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Recorded reason a task was reopened; anchors the repair slice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reopen_reason: Option<String>,
    /// Projection-only plan nodes derived from the authoritative task record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_nodes: Vec<PlanNodeDraft>,
}

fn default_representation() -> RetrievalRepresentation {
    RetrievalRepresentation::Search
}

impl RetrievalRequest {
    /// Derive a request from the authoritative task record at its current stage.
    ///
    /// The role and representation come from [`stage_retrieval`], so a stage
    /// change changes what the map is asked for without the runtime hardcoding a
    /// role per call site.
    pub fn from_task(task: &SessionTask) -> Self {
        let stage = stage_retrieval(&task.stage);
        Self {
            objective: task.objective.clone(),
            stage: task.stage.clone(),
            workspace_root: task.workspace_target.clone(),
            seed_files: Vec::new(),
            seed_symbols: Vec::new(),
            changed_files: task.artifacts.clone(),
            role: stage.role,
            representation: stage.representation,
            token_budget: None,
            task_id: Some(task.task_id.clone()),
            reopen_reason: task.reopen_reason.clone(),
            plan_nodes: project_task_to_plan_nodes(task),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalResponse {
    pub candidates: Vec<RetrievalCandidate>,
    pub provider: String,
    /// Representation the provider was asked for; recorded for observability.
    #[serde(default = "default_representation")]
    pub representation: RetrievalRepresentation,
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
            representation: RetrievalRepresentation::Search,
            plan_signals: Vec::new(),
        }
    }

    /// Empty response for a specific representation slice.
    pub fn empty_for(provider: impl Into<String>, representation: RetrievalRepresentation) -> Self {
        Self {
            candidates: Vec::new(),
            provider: provider.into(),
            representation,
            plan_signals: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_retrieval_matches_the_map_stage_table() {
        let cases = [
            (
                TaskStage::Selected,
                MapRole::Locate,
                RetrievalRepresentation::Search,
            ),
            (
                TaskStage::ContextPrepared,
                MapRole::Slice,
                RetrievalRepresentation::Delta,
            ),
            (
                TaskStage::Implementing,
                MapRole::Project,
                RetrievalRepresentation::Delta,
            ),
            (
                TaskStage::Verifying,
                MapRole::Locate,
                RetrievalRepresentation::Anchors,
            ),
            (
                TaskStage::Reviewing,
                MapRole::Project,
                RetrievalRepresentation::Review,
            ),
            (
                TaskStage::Repairing,
                MapRole::Project,
                RetrievalRepresentation::Impact,
            ),
            (
                TaskStage::Completed,
                MapRole::Reconcile,
                RetrievalRepresentation::Reconcile,
            ),
        ];
        for (stage, role, representation) in cases {
            let mapped = stage_retrieval(&stage);
            assert_eq!(mapped.role, role, "role for {stage:?}");
            assert_eq!(
                mapped.representation, representation,
                "representation for {stage:?}"
            );
        }
    }
}
