//! Projection of the authoritative task record into `scopemux` plan nodes.
//!
//! Plan nodes are projection-only: they describe intended code and never
//! advance task stage, verification, review, or completion. The durable task
//! record stays authoritative (`invariants/task-state.md`); this module defines
//! which task fields map to which plan-node attributes so the runtime can hand
//! intent to a retrieval provider and receive reconciliation evidence back.
//!
//! Projection contract (task record -> plan-node draft):
//!
//! | Task field | Plan-node attribute |
//! | --- | --- |
//! | `task_id` | id prefix `plan:<task_id>:<slug>` (assigned by the provider) |
//! | `objective` | one node: title + rationale |
//! | `completion_criteria` | one node each: desired shape + rationale |
//! | `workspace_target` | `file_path` |
//! | `artifacts` | `file:` anchors |
//! | `reopen_reason` | one `refactor_opportunity` node: rationale |
//! | `stage` | lifecycle seed (see [`stage_lifecycle`]) |
//!
//! A criterion may name an expected symbol with a backticked token (for example
//! ``add a `parse` function``); that token becomes `projected_symbol` so a later
//! reconciliation can report the node as implemented.

use serde::{Deserialize, Serialize};

use crate::task::{SessionTask, TaskStage};

/// Kind of projected (target-state) plan node. Mirrors `scopemux-core`'s
/// `ProjectPlanNodeKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanNodeKind {
    NewSymbol,
    NewFile,
    NewModule,
    NewTest,
    ModifySymbol,
    Remove,
    Consolidate,
    RefactorOpportunity,
    ObservabilityPoint,
}

impl PlanNodeKind {
    /// Stable `scopemux-core` enum discriminant.
    pub fn as_ffi(self) -> i32 {
        match self {
            Self::NewSymbol => 0,
            Self::NewFile => 1,
            Self::NewModule => 2,
            Self::NewTest => 3,
            Self::ModifySymbol => 4,
            Self::Remove => 5,
            Self::Consolidate => 6,
            Self::RefactorOpportunity => 7,
            Self::ObservabilityPoint => 8,
        }
    }
}

/// Lifecycle of a planned node. Mirrors `scopemux-core`'s
/// `ProjectInfoBlockLifecycle`; parsed blocks use `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanLifecycle {
    None,
    Planned,
    InProgress,
    Implemented,
    Verified,
    Documented,
    Stale,
    Conflict,
    Abandoned,
}

impl PlanLifecycle {
    pub fn as_ffi(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Planned => 1,
            Self::InProgress => 2,
            Self::Implemented => 3,
            Self::Verified => 4,
            Self::Documented => 5,
            Self::Stale => 6,
            Self::Conflict => 7,
            Self::Abandoned => 8,
        }
    }

    pub fn from_ffi(value: i32) -> Self {
        match value {
            1 => Self::Planned,
            2 => Self::InProgress,
            3 => Self::Implemented,
            4 => Self::Verified,
            5 => Self::Documented,
            6 => Self::Stale,
            7 => Self::Conflict,
            8 => Self::Abandoned,
            _ => Self::None,
        }
    }

    /// True when the node is a divergence signal the runtime should re-plan on.
    pub fn is_divergence(self) -> bool {
        matches!(self, Self::Stale | Self::Conflict)
    }

    /// True when the projected code has been observed in parsed state.
    pub fn is_realized(self) -> bool {
        matches!(self, Self::Implemented | Self::Verified | Self::Documented)
    }
}

/// A projection-only plan node, ready to be materialized by a provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanNodeDraft {
    pub slug: String,
    pub kind: PlanNodeKind,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_shape: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projected_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub anchors: Vec<String>,
}

/// Lifecycle transition reported by reconciliation. Evidence only: the runtime
/// decides whether to act on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanReconciliationSignal {
    pub plan_node_id: String,
    pub previous: PlanLifecycle,
    pub current: PlanLifecycle,
}

/// Seed the plan lifecycle from the authoritative task stage.
///
/// This is a projection of where the task is, not a driver of it: the runtime
/// remains the only writer of stage and completion.
pub fn stage_lifecycle(stage: &TaskStage) -> PlanLifecycle {
    match stage {
        TaskStage::Selected | TaskStage::ContextPrepared => PlanLifecycle::Planned,
        TaskStage::Implementing
        | TaskStage::Verifying
        | TaskStage::Reviewing
        | TaskStage::Repairing => PlanLifecycle::InProgress,
        TaskStage::Completed => PlanLifecycle::Implemented,
    }
}

fn infer_kind(text: &str) -> PlanNodeKind {
    let lower = text.to_ascii_lowercase();
    if lower.contains("remove") || lower.contains("delete") || lower.contains("drop") {
        PlanNodeKind::Remove
    } else if lower.contains("test") {
        PlanNodeKind::NewTest
    } else if lower.contains("consolidat") {
        PlanNodeKind::Consolidate
    } else if lower.contains("refactor") || lower.contains("duplicate") {
        PlanNodeKind::RefactorOpportunity
    } else if lower.contains("log")
        || lower.contains("metric")
        || lower.contains("observability")
        || lower.contains("trace")
        || lower.contains("assert")
    {
        PlanNodeKind::ObservabilityPoint
    } else if lower.contains("add")
        || lower.contains("create")
        || lower.contains("introduce")
        || lower.contains("new ")
    {
        PlanNodeKind::NewSymbol
    } else {
        PlanNodeKind::ModifySymbol
    }
}

/// Extract a backticked identifier from criterion text, if one is present.
fn extract_symbol(text: &str) -> Option<String> {
    let start = text.find('`')?;
    let rest = &text[start + 1..];
    let end = rest.find('`')?;
    let candidate = rest[..end].trim();
    if !candidate.is_empty()
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':' || c == '.')
    {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = true;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "node".to_string()
    } else {
        slug
    }
}

/// Project a task record into plan-node drafts. Deterministic and side-effect
/// free; the runtime owns the task record.
pub fn project_task_to_plan_nodes(task: &SessionTask) -> Vec<PlanNodeDraft> {
    let mut drafts = Vec::new();
    let file_path = if task.workspace_target.is_empty() {
        None
    } else {
        Some(task.workspace_target.clone())
    };
    let anchors: Vec<String> = task
        .artifacts
        .iter()
        .map(|artifact| format!("file:{artifact}"))
        .collect();

    if !task.objective.trim().is_empty() {
        drafts.push(PlanNodeDraft {
            slug: "objective".to_string(),
            kind: infer_kind(&task.objective),
            title: task.objective.clone(),
            desired_shape: None,
            rationale: Some(format!("objective: {}", task.objective)),
            projected_symbol: extract_symbol(&task.objective),
            file_path: file_path.clone(),
            anchors: anchors.clone(),
        });
    }

    for (index, criterion) in task.completion_criteria.iter().enumerate() {
        if criterion.trim().is_empty() {
            continue;
        }
        drafts.push(PlanNodeDraft {
            slug: format!("criterion-{}", index + 1),
            kind: infer_kind(criterion),
            title: criterion.clone(),
            desired_shape: Some(criterion.clone()),
            rationale: Some(format!("completion criterion: {criterion}")),
            projected_symbol: extract_symbol(criterion),
            file_path: file_path.clone(),
            anchors: anchors.clone(),
        });
    }

    if let Some(reason) = task
        .reopen_reason
        .as_deref()
        .filter(|r| !r.trim().is_empty())
    {
        drafts.push(PlanNodeDraft {
            slug: format!("reopen-{}", slugify(reason)),
            kind: PlanNodeKind::RefactorOpportunity,
            title: reason.to_string(),
            desired_shape: None,
            rationale: Some(format!("reopen: {reason}")),
            projected_symbol: None,
            file_path: file_path.clone(),
            anchors,
        });
    }

    drafts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> SessionTask {
        let mut task = SessionTask::new(
            "add a `parse` function",
            vec![
                "implement `parse` for the parser".to_string(),
                "cover parse with a unit test".to_string(),
            ],
            "/ws/project",
            Vec::new(),
        );
        task.task_id = "task_demo".to_string();
        task.artifacts = vec!["/ws/project/src/lib.c".to_string()];
        task
    }

    #[test]
    fn projects_objective_and_criteria_with_anchors() {
        let drafts = project_task_to_plan_nodes(&task());
        assert_eq!(drafts.len(), 3);

        let objective = &drafts[0];
        assert_eq!(objective.slug, "objective");
        assert_eq!(objective.kind, PlanNodeKind::NewSymbol);
        assert_eq!(objective.projected_symbol.as_deref(), Some("parse"));
        assert_eq!(objective.file_path.as_deref(), Some("/ws/project"));

        let first = &drafts[1];
        assert_eq!(first.slug, "criterion-1");
        assert_eq!(
            first.desired_shape.as_deref(),
            Some("implement `parse` for the parser")
        );
        assert_eq!(
            first.anchors,
            vec!["file:/ws/project/src/lib.c".to_string()]
        );

        let second = &drafts[2];
        assert_eq!(second.kind, PlanNodeKind::NewTest);
    }

    #[test]
    fn reopen_reason_adds_a_refactor_node() {
        let mut task = task();
        task.reopen_reason = Some("the parse function regressed".to_string());
        let drafts = project_task_to_plan_nodes(&task);
        assert_eq!(drafts.len(), 4);
        assert_eq!(drafts[3].kind, PlanNodeKind::RefactorOpportunity);
        assert!(drafts[3].slug.starts_with("reopen-"));
    }

    #[test]
    fn stage_lifecycle_is_projection_only() {
        assert_eq!(
            stage_lifecycle(&TaskStage::Selected),
            PlanLifecycle::Planned
        );
        assert_eq!(
            stage_lifecycle(&TaskStage::Implementing),
            PlanLifecycle::InProgress
        );
        assert_eq!(
            stage_lifecycle(&TaskStage::Completed),
            PlanLifecycle::Implemented
        );
    }
}
