use opencode_session::{
    MessageRole, Session, SessionStatus, SessionTask, TaskAction, TaskReviewStatus, TaskStage,
    TaskStageError, TaskVerificationStatus,
};

#[test]
fn test_session_creation() {
    let session = Session::new("test-project", "/test/directory");

    assert!(session.id.starts_with("ses_"));
    assert!(session.messages.is_empty());
    assert_eq!(session.project_id, "test-project");
    assert_eq!(session.directory, "/test/directory");
}

#[test]
fn test_session_add_user_message() {
    let mut session = Session::new("test-project", "/test/directory");

    session.add_user_message("Hello, world!");

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, MessageRole::User);
}

#[test]
fn test_session_add_assistant_message() {
    let mut session = Session::new("test-project", "/test/directory");

    session.add_user_message("Hello");
    session.add_assistant_message();

    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.messages[0].role, MessageRole::User);
    assert_eq!(session.messages[1].role, MessageRole::Assistant);
}

#[test]
fn test_session_child_creation() {
    let parent = Session::new("test-project", "/test/directory");
    let child = Session::child(&parent);

    assert!(child.parent_id.is_some());
    assert_eq!(child.parent_id.unwrap(), parent.id);
    assert_eq!(child.project_id, parent.project_id);
    assert_eq!(child.directory, parent.directory);
}

#[test]
fn test_session_default_title() {
    let session = Session::new("test-project", "/test/directory");

    assert!(session.is_default_title());

    let mut session_with_title = Session::new("test-project", "/test/directory");
    session_with_title.title = "Custom Title".to_string();

    assert!(!session_with_title.is_default_title());
}

#[test]
fn test_session_forked_title() {
    let mut session = Session::new("test-project", "/test/directory");
    session.title = "My Session".to_string();

    let forked = session.get_forked_title();
    assert_eq!(forked, "My Session (fork #1)");

    session.title = "My Session (fork #1)".to_string();
    let forked2 = session.get_forked_title();
    assert_eq!(forked2, "My Session (fork #2)");
}

#[test]
fn test_session_touch_updates_timestamp() {
    let mut session = Session::new("test-project", "/test/directory");
    let original_time = session.time.updated;

    std::thread::sleep(std::time::Duration::from_millis(10));
    session.touch();

    assert!(session.time.updated >= original_time);
}

#[test]
fn test_session_message_id() {
    let mut session = Session::new("test-project", "/test/directory");

    session.add_user_message("Test message");
    assert!(session.messages[0].id.len() > 0);
    assert_eq!(session.messages[0].role, MessageRole::User);
}

#[test]
fn test_task_transitions_require_verification_and_review() {
    let mut session = Session::new("test-project", "/test/directory");
    session.set_task(SessionTask::new(
        "Enforce runtime gates",
        vec!["Verification passed".to_string()],
        "/test/directory",
        vec!["cargo test -p opencode-session".to_string()],
    ));

    session.advance_task(TaskAction::PrepareContext).unwrap();
    session.advance_task(TaskAction::StartImplementing).unwrap();

    let err = session.complete().unwrap_err();
    assert_eq!(err, TaskStageError::VerificationRequired);

    session.advance_task(TaskAction::StartVerifying).unwrap();
    session
        .advance_task(TaskAction::RecordVerification {
            status: TaskVerificationStatus::Passed,
            notes: Some("session tests passed".to_string()),
        })
        .unwrap();

    let err = session.complete().unwrap_err();
    assert_eq!(err, TaskStageError::ReviewApprovalRequired);

    session
        .advance_task(TaskAction::RecordReview {
            status: TaskReviewStatus::Approved,
            notes: Some("review approved".to_string()),
        })
        .unwrap();

    assert_eq!(session.task.as_ref().unwrap().stage, TaskStage::Completed);
    assert_eq!(session.status, SessionStatus::Completed);
    session.complete().unwrap();
}

#[test]
fn test_failed_verification_reopens_into_repairing() {
    let mut session = Session::new("test-project", "/test/directory");
    session.set_task(SessionTask::new(
        "Enforce runtime gates",
        vec!["Verification passed".to_string()],
        "/test/directory",
        vec!["cargo test -p opencode-session".to_string()],
    ));

    session.advance_task(TaskAction::PrepareContext).unwrap();
    session.advance_task(TaskAction::StartImplementing).unwrap();
    session.advance_task(TaskAction::StartVerifying).unwrap();
    session
        .advance_task(TaskAction::RecordVerification {
            status: TaskVerificationStatus::Failed,
            notes: Some("cargo test failed".to_string()),
        })
        .unwrap();

    let task = session.task.as_ref().unwrap();
    assert_eq!(task.stage, TaskStage::Repairing);
    assert_eq!(task.reopen_reason.as_deref(), Some("cargo test failed"));

    session
        .advance_task(TaskAction::RestartImplementation)
        .unwrap();
    assert_eq!(
        session.task.as_ref().unwrap().stage,
        TaskStage::Implementing
    );
}

#[test]
fn test_blocking_review_reopens_into_repairing() {
    let mut session = Session::new("test-project", "/test/directory");
    session.set_task(SessionTask::new(
        "Enforce runtime gates",
        vec!["Verification passed".to_string()],
        "/test/directory",
        vec!["cargo test -p opencode-session".to_string()],
    ));

    session.advance_task(TaskAction::PrepareContext).unwrap();
    session.advance_task(TaskAction::StartImplementing).unwrap();
    session.advance_task(TaskAction::StartVerifying).unwrap();
    session
        .advance_task(TaskAction::RecordVerification {
            status: TaskVerificationStatus::Passed,
            notes: Some("cargo test passed".to_string()),
        })
        .unwrap();
    session
        .advance_task(TaskAction::RecordReview {
            status: TaskReviewStatus::ChangesRequested,
            notes: Some("fix blocking issue".to_string()),
        })
        .unwrap();

    let task = session.task.as_ref().unwrap();
    assert_eq!(task.stage, TaskStage::Repairing);
    assert_eq!(task.reopen_reason.as_deref(), Some("fix blocking issue"));
    assert_eq!(session.status, SessionStatus::Active);
}
