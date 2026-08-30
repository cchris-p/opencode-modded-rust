use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStage {
    Selected,
    ContextPrepared,
    Implementing,
    Verifying,
    Reviewing,
    Repairing,
    Completed,
}

impl Default for TaskStage {
    fn default() -> Self {
        Self::Selected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskVerificationStatus {
    NotRun,
    Passed,
    Failed,
    Incomplete,
}

impl Default for TaskVerificationStatus {
    fn default() -> Self {
        Self::NotRun
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskReviewStatus {
    NotReviewed,
    Approved,
    ChangesRequested,
}

impl Default for TaskReviewStatus {
    fn default() -> Self {
        Self::NotReviewed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionTask {
    pub task_id: String,
    pub objective: String,
    #[serde(default)]
    pub completion_criteria: Vec<String>,
    pub workspace_target: String,
    #[serde(default)]
    pub stage: TaskStage,
    #[serde(default)]
    pub verification_plan: Vec<String>,
    #[serde(default)]
    pub verification_status: TaskVerificationStatus,
    #[serde(default)]
    pub verification_notes: Option<String>,
    #[serde(default)]
    pub review_status: TaskReviewStatus,
    #[serde(default)]
    pub review_notes: Option<String>,
    #[serde(default)]
    pub reopen_reason: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl SessionTask {
    pub fn new(
        objective: impl Into<String>,
        completion_criteria: Vec<String>,
        workspace_target: impl Into<String>,
        verification_plan: Vec<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            task_id: format!("task_{}", Uuid::new_v4().simple()),
            objective: objective.into(),
            completion_criteria,
            workspace_target: workspace_target.into(),
            stage: TaskStage::Selected,
            verification_plan,
            verification_status: TaskVerificationStatus::NotRun,
            verification_notes: None,
            review_status: TaskReviewStatus::NotReviewed,
            review_notes: None,
            reopen_reason: None,
            artifacts: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }
}
