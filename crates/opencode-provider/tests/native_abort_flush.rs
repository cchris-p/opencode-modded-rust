//! Characterization + regression coverage for the DeepSeek native tool-call
//! extractor when a stream is cut short (budget/timeout/abort) or a native block
//! is left unterminated at finish (BUG-063).
//!
//! Both the stateful parser used by the re-routed providers and the legacy
//! parser used by `novita-ai` were probed and behave identically:
//! - no terminal `finish_reason`/`[DONE]` -> the buffered native call is dropped,
//!   the raw tokens never reach the assistant text;
//! - `finish_reason`/`[DONE]` after an *unterminated* native block -> the partial
//!   native tokens still leak into the assistant text (tracked below).

use futures::StreamExt;
use opencode_provider::{openai_compat_sse_stream, StreamEvent};

const CB: &str = "<\u{ff5c}tool\u{2581}calls\u{2581}begin\u{ff5c}>";
const CBE: &str = "<\u{ff5c}tool\u{2581}call\u{2581}begin\u{ff5c}>";
const SEP: &str = "<\u{ff5c}tool\u{2581}sep\u{ff5c}>";
const CE: &str = "<\u{ff5c}tool\u{2581}call\u{2581}end\u{ff5c}>";
const CS: &str = "<\u{ff5c}tool\u{2581}calls\u{2581}end\u{ff5c}>";

fn native_call(name: &str, args: &str) -> String {
    format!("{CB}{CBE}function{SEP}{name}\n```json\n{args}\n```{CE}{CS}")
}

fn content_frame(text: &str) -> String {
    let payload = serde_json::json!({"choices":[{"delta":{"content": text}}]}).to_string();
    format!("data: {payload}\n\n")
}

fn finish_frame() -> String {
    "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n".to_string()
}

fn replay(body: &str) -> Vec<StreamEvent> {
    let chunks: Vec<Result<Vec<u8>, reqwest::Error>> = vec![Ok(body.as_bytes().to_vec())];
    let stream = openai_compat_sse_stream(futures::stream::iter(chunks));
    futures::executor::block_on(stream.map(|e| e.expect("no transport error")).collect())
}

fn text(events: &[StreamEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::TextDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect()
}

fn calls(events: &[StreamEvent]) -> Vec<(String, serde_json::Value)> {
    events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::ToolCallEnd { name, input, .. } => Some((name.clone(), input.clone())),
            _ => None,
        })
        .collect()
}

fn assert_no_raw_native_tokens(events: &[StreamEvent]) {
    let visible = text(events);
    assert!(
        !visible.contains('\u{ff5c}') && !visible.contains('\u{2581}'),
        "raw native tool-call tokens leaked into assistant text: {visible:?}"
    );
}

#[test]
fn complete_native_block_then_finish_emits_tool_call() {
    let body = format!(
        "{}{}{}",
        content_frame("Run it:"),
        content_frame(&native_call("bash", "{\"command\":\"echo hi\"}")),
        finish_frame()
    );
    let events = replay(&body);
    assert_eq!(text(&events), "Run it:");
    assert_eq!(
        calls(&events),
        vec![(
            "bash".to_string(),
            serde_json::json!({"command": "echo hi"})
        )]
    );
}

#[test]
fn complete_native_block_without_finish_does_not_leak() {
    // Simulates a budget/timeout/abort cut after a complete native block: no
    // finish_reason and no [DONE]. The buffered call is dropped, and crucially
    // no raw tokens reach the assistant text.
    let body = format!(
        "{}{}",
        content_frame("Run it:"),
        content_frame(&native_call("bash", "{\"command\":\"echo hi\"}"))
    );
    let events = replay(&body);
    assert_eq!(text(&events), "Run it:");
    assert!(calls(&events).is_empty());
    assert_no_raw_native_tokens(&events);
}

#[test]
fn partial_native_block_without_finish_does_not_leak() {
    let partial = format!("{CB}{CBE}function{SEP}bash\n```json\n{{\"command\":\"echo hi\"}}");
    let body = format!("{}{}", content_frame("Run it:"), content_frame(&partial));
    let events = replay(&body);
    assert_eq!(text(&events), "Run it:");
    assert!(calls(&events).is_empty());
    assert_no_raw_native_tokens(&events);
}

#[test]
#[ignore = "BUG-063 residual: an unterminated native block still leaks its partial tokens when a terminal frame arrives"]
fn partial_native_block_then_finish_must_not_leak() {
    let partial = format!("{CB}{CBE}function{SEP}bash\n```json\n{{\"command\":\"echo hi\"}}");
    let body = format!(
        "{}{}{}",
        content_frame("Run it:"),
        content_frame(&partial),
        finish_frame()
    );
    let events = replay(&body);
    assert_no_raw_native_tokens(&events);
}
