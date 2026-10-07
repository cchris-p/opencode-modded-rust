//! Deterministic replay of captured DeepSeek SSE frames (EPIC-001 / QA-002).
//!
//! Each fixture is a byte-accurate OpenAI-compatible streaming body. It is fed
//! through the real stateful parser (`openai_compat_sse_stream`) so a regression
//! in reasoning capture, split tool-call assembly, or terminal events is caught
//! offline without a network call.

use futures::StreamExt;
use opencode_provider::{openai_compat_sse_stream, StreamEvent};

const REASONING_TEXT_STOP: &str = include_str!("fixtures/deepseek/reasoning_text_stop.sse");
const REASONING_TOOLCALL_SPLIT_ARGS: &str =
    include_str!("fixtures/deepseek/reasoning_toolcall_split_args.sse");
const PARALLEL_TOOLCALLS_SPLIT_ARGS: &str =
    include_str!("fixtures/deepseek/parallel_toolcalls_split_args.sse");
const USAGE_REASONING_CONTENT: &str = include_str!("fixtures/deepseek/usage_reasoning_content.sse");
const NO_FINISH_NO_DONE: &str = include_str!("fixtures/deepseek/no_finish_no_done.sse");
const REASONING_ONLY_NO_TERMINAL: &str =
    include_str!("fixtures/deepseek/reasoning_only_no_terminal.sse");

fn replay_with_chunks(chunks: Vec<Vec<u8>>) -> Vec<StreamEvent> {
    let chunks: Vec<Result<Vec<u8>, reqwest::Error>> = chunks.into_iter().map(Ok).collect();
    let stream = openai_compat_sse_stream(futures::stream::iter(chunks));
    futures::executor::block_on(
        stream
            .map(|event| event.expect("fixture stream should not surface a transport error"))
            .collect(),
    )
}

fn replay(bytes: &str) -> Vec<StreamEvent> {
    replay_with_chunks(vec![bytes.as_bytes().to_vec()])
}

fn reasoning_text(events: &[StreamEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ReasoningDelta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn text(events: &[StreamEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::TextDelta(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn tool_starts(events: &[StreamEvent]) -> Vec<(String, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ToolCallStart { id, name } => Some((id.clone(), name.clone())),
            _ => None,
        })
        .collect()
}

fn tool_deltas(events: &[StreamEvent]) -> Vec<(String, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ToolCallDelta { id, input } => Some((id.clone(), input.clone())),
            _ => None,
        })
        .collect()
}

fn finish_reasons(events: &[StreamEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::FinishStep { finish_reason, .. } => Some(finish_reason.clone()),
            _ => None,
        })
        .collect()
}

fn has_done(events: &[StreamEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, StreamEvent::Done))
}

fn has_reasoning_start_and_end(events: &[StreamEvent]) -> bool {
    let starts = events
        .iter()
        .filter(|event| matches!(event, StreamEvent::ReasoningStart { .. }))
        .count();
    let ends = events
        .iter()
        .filter(|event| matches!(event, StreamEvent::ReasoningEnd { .. }))
        .count();
    starts == 1 && ends == 1
}

#[test]
fn reasoning_then_text_then_stop_parses_cleanly() {
    let events = replay(REASONING_TEXT_STOP);

    assert_eq!(reasoning_text(&events), "Let me think about it.");
    assert!(has_reasoning_start_and_end(&events));
    assert_eq!(text(&events), "Hello world");
    assert_eq!(finish_reasons(&events), vec![Some("stop".to_string())]);
    assert!(has_done(&events));
}

#[test]
fn reasoning_then_split_toolcall_joins_into_one_call() {
    let events = replay(REASONING_TOOLCALL_SPLIT_ARGS);

    assert_eq!(reasoning_text(&events), "I should read the file.");
    assert_eq!(
        tool_starts(&events),
        vec![("call_00_abc".to_string(), "read".to_string())]
    );
    let deltas = tool_deltas(&events);
    assert_eq!(deltas.len(), 2, "both argument fragments should be kept");
    assert!(
        deltas.iter().all(|(id, _)| id == "call_00_abc"),
        "argument fragments must join the real call id, got {deltas:?}"
    );
    assert_eq!(
        deltas
            .iter()
            .map(|(_, input)| input.clone())
            .collect::<String>(),
        "{\"filePath\":\"src/lib.rs\"}"
    );
    assert_eq!(
        finish_reasons(&events),
        vec![Some("tool-calls".to_string())]
    );
    assert!(has_done(&events));
}

#[test]
fn parallel_toolcalls_keep_distinct_ids() {
    let events = replay(PARALLEL_TOOLCALLS_SPLIT_ARGS);

    assert_eq!(
        tool_starts(&events),
        vec![
            ("call_a".to_string(), "read".to_string()),
            ("call_b".to_string(), "grep".to_string()),
        ]
    );
    let deltas = tool_deltas(&events);
    assert_eq!(
        deltas,
        vec![
            ("call_a".to_string(), "{\"filePath\":\"a\"}".to_string()),
            ("call_b".to_string(), "{\"pattern\":\"x\"}".to_string()),
        ]
    );
    assert_eq!(
        finish_reasons(&events),
        vec![Some("tool-calls".to_string())]
    );
    assert!(has_done(&events));
}

#[test]
fn usage_and_reasoning_in_one_payload() {
    let events = replay(USAGE_REASONING_CONTENT);

    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::Usage {
            prompt_tokens: 1,
            completion_tokens: 2
        }
    )));
    assert_eq!(reasoning_text(&events), "r");
    assert!(has_reasoning_start_and_end(&events));
    assert_eq!(text(&events), "c");
    assert!(has_done(&events));
}

#[test]
fn body_close_without_finish_has_no_terminal_event() {
    let events = replay(NO_FINISH_NO_DONE);

    assert!(finish_reasons(&events).is_empty());
    assert!(!has_done(&events));
    assert_eq!(text(&events), "partial answer");
}

#[test]
fn reasoning_only_has_no_terminal_event() {
    let events = replay(REASONING_ONLY_NO_TERMINAL);

    assert_eq!(reasoning_text(&events), "thinking more and more");
    assert!(finish_reasons(&events).is_empty());
    assert!(!has_done(&events));
}

#[test]
fn chunk_boundaries_do_not_change_parsed_events() {
    let fixture = REASONING_TOOLCALL_SPLIT_ARGS;
    let expected = serde_json::to_value(replay(fixture)).expect("events serialize");
    let bytes = fixture.as_bytes();

    for split in 1..bytes.len() {
        let events = replay_with_chunks(vec![bytes[..split].to_vec(), bytes[split..].to_vec()]);
        let actual = serde_json::to_value(&events).expect("events serialize");
        assert_eq!(
            actual, expected,
            "splitting the fixture at byte {split} changed the parsed events"
        );
    }
}
