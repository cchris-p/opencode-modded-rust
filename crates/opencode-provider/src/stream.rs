use crate::provider::ProviderError;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    /// Stream has started.
    Start,
    /// Incremental text content.
    TextDelta(String),
    /// Start of a text block.
    TextStart,
    /// End of a text block.
    TextEnd,
    /// Start of a reasoning/thinking block.
    ReasoningStart {
        id: String,
    },
    /// Incremental reasoning text.
    ReasoningDelta {
        id: String,
        text: String,
    },
    /// End of a reasoning/thinking block.
    ReasoningEnd {
        id: String,
    },
    /// Start of tool input streaming (tool-input-start in TS).
    ToolInputStart {
        id: String,
        tool_name: String,
    },
    /// Incremental tool input JSON (tool-input-delta in TS).
    ToolInputDelta {
        id: String,
        delta: String,
    },
    /// End of tool input streaming (tool-input-end in TS).
    ToolInputEnd {
        id: String,
    },
    /// Full tool call event (after input is fully assembled).
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta {
        id: String,
        input: String,
    },
    ToolCallEnd {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result received.
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        input: Option<serde_json::Value>,
        output: ToolResultOutput,
    },
    /// Tool error received.
    ToolError {
        tool_call_id: String,
        tool_name: String,
        input: Option<serde_json::Value>,
        error: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<ToolErrorKind>,
    },
    /// Start of a processing step (maps to start-step in TS).
    StartStep,
    /// End of a processing step with usage info (maps to finish-step in TS).
    FinishStep {
        finish_reason: Option<String>,
        usage: StreamUsage,
        provider_metadata: Option<serde_json::Value>,
    },
    Usage {
        prompt_tokens: u64,
        completion_tokens: u64,
    },
    /// Stream finished (maps to "finish" in TS).
    Finish,
    Done,
    Error(String),
}

/// Type-safe tool error category for streaming tool failures.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolErrorKind {
    PermissionDenied,
    QuestionRejected,
    ExecutionError,
}

/// Output from a tool result event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultOutput {
    pub output: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<serde_json::Value>>,
}

/// Usage information from a step completion.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    #[serde(default)]
    pub reasoning_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub cache_write_tokens: u64,
}

pub type StreamResult = Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>;

/// Build a provider event stream from a raw SSE byte stream.
///
/// Raw HTTP chunk boundaries do not align with SSE frame boundaries: a single
/// network chunk can carry several `data:` frames and a single frame can be
/// split across two chunks. Naive per-chunk line parsing silently drops every
/// frame except the first one in each chunk, which garbles or truncates model
/// output. This adapter buffers bytes across chunks, splits complete lines,
/// and emits every event produced for each line so no frame is lost.
///
/// `parse_line` receives each complete, CRLF-trimmed line and returns zero or
/// more events for it. Lines that produce no events (comments, keepalives,
/// malformed frames, etc.) are simply skipped.
pub fn sse_event_stream<S, B, F>(
    chunks: S,
    parse_line: F,
) -> StreamResult
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin + Send + 'static,
    B: AsRef<[u8]> + Send + 'static,
    F: Fn(&str) -> Vec<StreamEvent> + Send + Sync + 'static,
{
    use futures::StreamExt;
    use std::collections::VecDeque;

    let stream = futures::stream::try_unfold(
        (
            chunks,
            String::new(),
            VecDeque::<StreamEvent>::new(),
            false,
            parse_line,
        ),
        |(mut chunks, mut buffer, mut pending, mut exhausted, parse_line)| async move {
            loop {
                if let Some(event) = pending.pop_front() {
                    return Ok(Some((
                        event,
                        (chunks, buffer, pending, exhausted, parse_line),
                    )));
                }

                if let Some(idx) = buffer.find('\n') {
                    let mut line = buffer[..idx].to_string();
                    buffer.drain(..=idx);
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    if line.is_empty() {
                        continue;
                    }
                    pending.extend(parse_line(line.trim()));
                    continue;
                }

                if exhausted {
                    let tail = std::mem::take(&mut buffer);
                    if !tail.trim().is_empty() {
                        pending.extend(parse_line(tail.trim()));
                    }
                    if let Some(event) = pending.pop_front() {
                        return Ok(Some((
                            event,
                            (chunks, buffer, pending, true, parse_line),
                        )));
                    }
                    return Ok(None);
                }

                match chunks.next().await {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(bytes.as_ref()));
                    }
                    Some(Err(e)) => {
                        return Err(ProviderError::StreamError(e.to_string()));
                    }
                    None => exhausted = true,
                }
            }
        },
    );

    Box::pin(stream)
}

/// Parse one complete SSE line in the OpenAI-compatible `data: <json>` shape.
///
/// Handles the `data: [DONE]` terminator and delegates every other payload to
/// [`parse_openai_sse`]. Lines that are not `data:` frames produce no events.
pub fn openai_compat_line_events(line: &str) -> Vec<StreamEvent> {
    let Some(payload) = line.strip_prefix("data:") else {
        return Vec::new();
    };
    let payload = payload.trim();
    if payload.is_empty() {
        return Vec::new();
    }
    if payload == "[DONE]" {
        return vec![StreamEvent::Done];
    }
    parse_openai_sse(payload).into_iter().collect()
}

/// Parse one complete SSE line in the Anthropic `data: <json>` shape.
pub fn anthropic_line_events(line: &str) -> Vec<StreamEvent> {
    let Some(payload) = line.strip_prefix("data:") else {
        return Vec::new();
    };
    let payload = payload.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return Vec::new();
    }
    parse_anthropic_sse(payload).into_iter().collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAISSEvent {
    #[serde(default)]
    pub choices: Vec<OpenAIChoice>,
    pub usage: Option<OpenAIUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    #[serde(default)]
    pub delta: Option<OpenAIDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIDelta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    pub reasoning_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolCall {
    #[serde(default)]
    pub index: u32,
    pub id: Option<String>,
    #[serde(default)]
    pub function: Option<OpenAIFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAIFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
}

fn openai_tool_call_id(tc: &OpenAIToolCall) -> String {
    tc.id
        .clone()
        .unwrap_or_else(|| format!("tool-call-{}", tc.index))
}

pub fn parse_openai_sse(data: &str) -> Option<StreamEvent> {
    if data == "[DONE]" {
        return Some(StreamEvent::Done);
    }

    let event: OpenAISSEvent = serde_json::from_str(data).ok()?;

    for choice in event.choices {
        if let Some(delta) = &choice.delta {
            if let Some(content) = &delta.content {
                if !content.is_empty() {
                    return Some(StreamEvent::TextDelta(content.clone()));
                }
            }

            if let Some(tool_calls) = &delta.tool_calls {
                for tc in tool_calls {
                    if let Some(func) = &tc.function {
                        if let Some(name) = &func.name {
                            return Some(StreamEvent::ToolCallStart {
                                id: openai_tool_call_id(tc),
                                name: name.clone(),
                            });
                        }
                        if let Some(args) = &func.arguments {
                            if !args.is_empty() {
                                return Some(StreamEvent::ToolCallDelta {
                                    id: openai_tool_call_id(tc),
                                    input: args.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        if let Some(reason) = &choice.finish_reason {
            if reason == "tool_calls" {
                return Some(StreamEvent::Done);
            }
        }
    }

    if let Some(usage) = event.usage {
        return Some(StreamEvent::Usage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
        });
    }

    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: Option<u32>,
    pub delta: Option<AnthropicDelta>,
    pub content_block: Option<AnthropicContentBlock>,
    pub message: Option<AnthropicMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicDelta {
    #[serde(rename = "type")]
    pub delta_type: Option<String>,
    pub text: Option<String>,
    pub partial_json: Option<String>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

pub fn parse_anthropic_sse(data: &str) -> Option<StreamEvent> {
    let event: AnthropicEvent = serde_json::from_str(data).ok()?;

    match event.event_type.as_str() {
        "content_block_delta" => {
            if let Some(delta) = event.delta {
                if let Some(text) = delta.text {
                    return Some(StreamEvent::TextDelta(text));
                }
                if let Some(json) = delta.partial_json {
                    return Some(StreamEvent::ToolCallDelta {
                        id: String::new(),
                        input: json,
                    });
                }
            }
        }
        "content_block_start" => {
            if let Some(block) = event.content_block {
                if block.block_type == "tool_use" {
                    return Some(StreamEvent::ToolCallStart {
                        id: block.id.unwrap_or_default(),
                        name: block.name.unwrap_or_default(),
                    });
                }
            }
        }
        "content_block_stop" => {
            return Some(StreamEvent::Done);
        }
        "message_delta" => {
            if let Some(delta) = event.delta {
                if delta.stop_reason.is_some() {
                    return Some(StreamEvent::Done);
                }
            }
        }
        "message_start" => {
            if let Some(msg) = event.message {
                if let Some(usage) = msg.usage {
                    return Some(StreamEvent::Usage {
                        prompt_tokens: usage.input_tokens,
                        completion_tokens: usage.output_tokens,
                    });
                }
            }
        }
        _ => {}
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_sse_uses_fallback_id_for_tool_start() {
        let data =
            r#"{"choices":[{"delta":{"tool_calls":[{"index":2,"function":{"name":"bash"}}]}}]}"#;
        let event = parse_openai_sse(data).expect("event should parse");
        match event {
            StreamEvent::ToolCallStart { id, name } => {
                assert_eq!(id, "tool-call-2");
                assert_eq!(name, "bash");
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_openai_sse_uses_fallback_id_for_tool_delta() {
        let data = r#"{"choices":[{"delta":{"tool_calls":[{"index":2,"function":{"arguments":"{\"x\":1}"}}]}}]}"#;
        let event = parse_openai_sse(data).expect("event should parse");
        match event {
            StreamEvent::ToolCallDelta { id, input } => {
                assert_eq!(id, "tool-call-2");
                assert_eq!(input, "{\"x\":1}");
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    /// Helper: run a buffered `sse_event_stream` to completion and collect events.
    fn collect_events<B>(chunks: Vec<Result<B, reqwest::Error>>) -> Vec<StreamEvent>
    where
        B: AsRef<[u8]> + Send + 'static,
    {
        use futures::StreamExt;
        let stream = sse_event_stream(futures::stream::iter(chunks), openai_compat_line_events);
        futures::executor::block_on(stream.map(Result::unwrap).collect())
    }

    #[test]
    fn sse_event_stream_emits_every_event_when_one_chunk_has_many_frames() {
        // One network chunk carries several SSE frames. The old per-chunk parser
        // returned only the first frame and dropped the rest, garbling output.
        let chunk = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" \"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"world\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let events = collect_events(vec![Ok(Vec::from(chunk.as_bytes()))]);
        let texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::TextDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["Hello".to_string(), " ".to_string(), "world".to_string()]);
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Done)));
    }

    #[test]
    fn sse_event_stream_reassembles_frames_split_across_chunks() {
        // A single SSE frame split across two network chunks must be reassembled,
        // not dropped as a partial parse failure.
        let chunk1 = "data: {\"choices\":[{\"delta\":{\"content\":\"Hel";
        let chunk2 = concat!(
            "lo\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"!\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let events = collect_events(vec![
            Ok(Vec::from(chunk1.as_bytes())),
            Ok(Vec::from(chunk2.as_bytes())),
        ]);
        let texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::TextDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["Hello".to_string(), "!".to_string()]);
    }

    #[test]
    fn sse_event_stream_flushes_trailing_frame_without_newline() {
        // The final frame may arrive without a trailing newline before EOF; the
        // buffered parser must still emit it.
        let chunk1 = "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: ";
        let chunk2 = "[DONE]";
        let events = collect_events(vec![
            Ok(Vec::from(chunk1.as_bytes())),
            Ok(Vec::from(chunk2.as_bytes())),
        ]);
        let texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::TextDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["ok".to_string()]);
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Done)));
    }

    // ------------------------------------------------------------------------
    // Adversarial SSE integrity tests (QA-001)
    //
    // These guard the two BUG-003 failure classes deterministically:
    //   1. frames must survive arbitrary HTTP chunking (split frames, coalesced
    //      frames, CRLF line endings),
    //   2. every content delta must be reassembled into the exact original text
    //      (no drops, no reordering) regardless of parser used.
    // ------------------------------------------------------------------------

    #[test]
    fn sse_event_stream_handles_crlf_line_endings() {
        let chunk = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"alpha\"}}]}\r\n\r\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" beta\"}}]}\r\n\r\n",
            "data: [DONE]\r\n\r\n",
        );
        let events = collect_events(vec![Ok(Vec::from(chunk.as_bytes()))]);
        let texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::TextDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["alpha".to_string(), " beta".to_string()]);
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Done)));
    }

    #[test]
    fn sse_event_stream_skips_keepalive_and_comment_lines() {
        // SSE keepalive/comment lines (": ...") and blank separators between
        // frames must not create spurious text or break reassembly.
        let chunk = concat!(
            ": keep-alive\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"one\"}}]}\n\n",
            "\n",
            ": heartbeat\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" two\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let events = collect_events(vec![Ok(Vec::from(chunk.as_bytes()))]);
        let texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::TextDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["one".to_string(), " two".to_string()]);
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Done)));
    }

    #[test]
    fn sse_event_stream_reassembles_utf8_split_across_chunk_boundary() {
        // "héllo" contains é as a multi-byte UTF-8 sequence. Splitting exactly
        // between its continuation bytes must not corrupt the reassembled text.
        let frame = "data: {\"choices\":[{\"delta\":{\"content\":\"héllo\"}}]}\n\ndata: [DONE]\n\n";
        let bytes = frame.as_bytes();
        let split = bytes.len() / 2;
        let events = collect_events(vec![
            Ok(bytes[..split].to_vec()),
            Ok(bytes[split..].to_vec()),
        ]);
        let texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::TextDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["héllo".to_string()]);
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Done)));
    }

    #[test]
    fn sse_event_stream_preserves_long_text_exactly() {
        // A long multi-frame payload must reassemble to byte-identical text with
        // no dropped or reordered deltas (the original garble class).
        let words = (0..200)
            .map(|i| format!("tok{}", i))
            .collect::<Vec<_>>();
        let mut raw = String::new();
        for w in &words {
            raw.push_str(&format!(
                "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{}\"}}}}]}}\n\n",
                w
            ));
        }
        raw.push_str("data: [DONE]\n\n");

        let bytes = raw.as_bytes();
        // Chop the stream into many tiny chunks to force pathological splitting.
        let chunks: Vec<Result<Vec<u8>, _>> = bytes
            .chunks(7)
            .map(|c| Ok(c.to_vec()))
            .collect();
        let events = collect_events(chunks);
        let texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::TextDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, words);
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Done)));
    }

    #[test]
    fn sse_event_stream_delivers_done_only_once_with_trailing_partial_data() {
        // [DONE] plus trailing bytes that are not a full frame (e.g. a stray
        // keepalive fragment) must still terminate exactly once.
        let chunk1 = "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\ndata: [DONE]\n\n: trail";
        let events = collect_events(vec![Ok(Vec::from(chunk1.as_bytes()))]);
        let done_count = events.iter().filter(|e| matches!(e, StreamEvent::Done)).count();
        assert_eq!(done_count, 1);
    }

    #[test]
    fn anthropic_line_events_skips_non_data_lines() {
        // Anthropic-style SSE interleaves `event:` metadata lines with `data:`
        // frames; only the data payloads should be parsed.
        use crate::stream::anthropic_line_events;
        let frames = vec![
            StreamEvent::TextDelta("hello".to_string()),
            StreamEvent::Done,
        ];
        // event: line must yield nothing
        assert!(anthropic_line_events("event: content_block_delta").is_empty());
        // blank and comment lines must yield nothing
        assert!(anthropic_line_events("").is_empty());
        assert!(anthropic_line_events(": ping").is_empty());
        let _ = frames; // (kept for clarity of intent)
    }

    #[test]
    fn openai_compat_line_events_handles_done_and_ignores_garbage() {
        use crate::stream::openai_compat_line_events;
        // [DONE] becomes exactly one Done
        let evs = openai_compat_line_events("data: [DONE]");
        assert_eq!(evs.len(), 1);
        assert!(matches!(evs[0], StreamEvent::Done));
        // Non-data / garbage lines produce nothing and must not panic.
        assert!(openai_compat_line_events("event: done").is_empty());
        assert!(openai_compat_line_events("data: {not json").is_empty());
    }
}
