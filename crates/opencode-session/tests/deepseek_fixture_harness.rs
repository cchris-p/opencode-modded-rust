//! DeepSeek wire-fixture replay through the real session prompt loop (EPIC-001 / QA-002).
//!
//! The provider replays captured DeepSeek SSE bytes through the same stateful
//! parser the product uses, and the loop is driven exactly as the server drives
//! it. The assertions cover the "every run reaches a terminal state" invariant:
//! a completed fixture must record a durable finish reason, and a stream that
//! closes early must still terminate the run instead of hanging.

use async_trait::async_trait;
use opencode_provider::{
    openai_compat_sse_stream, ChatRequest, ChatResponse, ModelInfo, Provider, ProviderError,
    StreamResult,
};
use opencode_session::prompt::{AgentParams, ModelRef, PartInput, PromptInput, SessionPrompt};
use opencode_session::{MessageRole, Session};
use std::sync::Arc;

const REASONING_TEXT_STOP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../opencode-provider/tests/fixtures/deepseek/reasoning_text_stop.sse"
));
const NO_FINISH_NO_DONE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../opencode-provider/tests/fixtures/deepseek/no_finish_no_done.sse"
));

fn deepseek_model() -> ModelInfo {
    ModelInfo {
        id: "deepseek-flash".to_string(),
        name: "DeepSeek Flash".to_string(),
        provider: "deepseek".to_string(),
        context_window: 128_000,
        max_output_tokens: 8192,
        supports_vision: false,
        supports_tools: true,
        cost_per_million_input: 0.0,
        cost_per_million_output: 0.0,
    }
}

/// Replays a captured DeepSeek SSE body as the provider stream.
struct FixtureProvider {
    model: ModelInfo,
    body: &'static str,
}

#[async_trait]
impl Provider for FixtureProvider {
    fn id(&self) -> &str {
        "deepseek"
    }

    fn name(&self) -> &str {
        "DeepSeek"
    }

    fn models(&self) -> Vec<ModelInfo> {
        vec![self.model.clone()]
    }

    fn get_model(&self, id: &str) -> Option<&ModelInfo> {
        (self.model.id == id).then_some(&self.model)
    }

    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Err(ProviderError::InvalidRequest(
            "chat() is not used by the fixture harness".to_string(),
        ))
    }

    async fn chat_stream(&self, _request: ChatRequest) -> Result<StreamResult, ProviderError> {
        let chunks: Vec<Result<Vec<u8>, reqwest::Error>> = vec![Ok(self.body.as_bytes().to_vec())];
        Ok(openai_compat_sse_stream(futures::stream::iter(chunks)))
    }
}

async fn run_fixture(body: &'static str) -> (bool, Session) {
    let prompt = SessionPrompt::default();
    let mut session = Session::new("proj", ".");
    let provider = Arc::new(FixtureProvider {
        model: deepseek_model(),
        body,
    });

    let input = PromptInput {
        session_id: session.id.clone(),
        message_id: None,
        model: Some(ModelRef {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-flash".to_string(),
        }),
        agent: None,
        no_reply: false,
        system: None,
        variant: None,
        parts: vec![PartInput::Text {
            text: "Say hello".to_string(),
        }],
        tools: None,
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        prompt.prompt_with_update_hook(
            input,
            &mut session,
            provider,
            None,
            Vec::new(),
            AgentParams::default(),
            None,
        ),
    )
    .await
    .expect("the prompt loop must terminate instead of hanging");

    (result.is_ok(), session)
}

fn last_assistant_text(session: &Session) -> String {
    session
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::Assistant))
        .map(|message| message.get_text())
        .unwrap_or_default()
}

#[tokio::test]
async fn reasoning_text_stop_fixture_completes_with_terminal_record() {
    let (ok, session) = run_fixture(REASONING_TEXT_STOP).await;

    assert!(ok, "a clean DeepSeek stop response must complete the turn");
    assert_eq!(last_assistant_text(&session), "Hello world");

    let assistant = session
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::Assistant))
        .expect("an assistant message should be present");
    assert_eq!(
        assistant
            .metadata
            .get("finish_reason")
            .and_then(|value| value.as_str()),
        Some("stop"),
        "a completed turn must persist a durable finish reason"
    );
}

#[tokio::test]
async fn body_close_without_finish_still_terminates() {
    let (ok, session) = run_fixture(NO_FINISH_NO_DONE).await;

    // The stream closes with no finish_reason and no [DONE]. The run must not
    // hang; it currently finalizes the partial message as complete.
    assert!(
        ok,
        "a body that closes early must terminate the run, not hang it"
    );
    assert_eq!(last_assistant_text(&session), "partial answer");

    // Documented gap (EPIC-001): a body that closes before a terminal event is
    // finalized without a durable finish reason, so a truncated reply is
    // indistinguishable from a completed one at the record level.
    let assistant = session
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::Assistant))
        .expect("an assistant message should be present");
    assert!(
        assistant.metadata.get("finish_reason").is_none(),
        "behavior changed: a body-close now records a finish reason; update EPIC-001"
    );
}
