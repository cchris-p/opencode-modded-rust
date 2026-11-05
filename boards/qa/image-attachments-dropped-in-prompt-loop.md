---
id: "START-036"
title: "Image/file attachments dropped in the prompt loop (no image reaches the model)"
priority: "P1"
type: "bug"
area: "START"
spec: ""
status: "qa"
created: "2026-11-05"
updated: "2026-11-05"
---

# Image/file attachments dropped in the prompt loop

## Summary

User file/image attachments submitted to `POST /session/{id}/prompt` (the
`PartInput::File` path, e.g. the VaM image-interpretation bridge) are stored on
the user message but **never reach the provider request**. The model receives
only the text part, so it reports that no image was attached.

Found while building image support for the VaM `image-interpretation` skill
(ORT profile `ort-novita-ai`, model `novita-ai/deepseek/deepseek-v4.1-flash`).

## Root causes

1. `crates/opencode-session/src/prompt.rs` `parts_to_content` (used by
   `build_chat_messages` in the main prompt loop) matches Text, ToolCall,
   ToolResult, and Reasoning, then `_ => None`. `PartType::File` is absent, so
   every attachment is silently dropped before the provider request.
2. `crates/opencode-provider/src/openai_chat.rs` `openai_user_content` only
   recognized `content_type` `"image_url"` / `"image"`. Internal attachments use
   `content_type` `"file"` with an image `media_type`, so even after (1) the
   image was not serialized as an OpenAI `image_url` content part.

## Fix

- `prompt.rs::parts_to_content`: add a `PartType::File { url, filename, mime }`
  arm producing a `ContentPart` with `content_type: "file"`,
  `image_url`, `filename`, and `media_type` (as the other converters already do).
- `openai_chat.rs::openai_user_content`: treat a `"file"` part whose
  `media_type` starts with `image/` as an image and emit an `image_url` part.
  Added regression test `user_image_file_part_is_sent_as_image_url`.

## Evidence (2026-11-05)

- Before: captured `/openai/chat/completions` body for a `/prompt` call with a
  `PartInput::File` data URL contained only the text message; `tools` were also
  absent on the `/stream` endpoint.
- After: the same call serializes user content as
  `[{"type":"text",...},{"type":"image_url","image_url":{"url":"data:image/..."}}]`.
- Vision verified end-to-end with a built binary: a 320x240 red PNG returns
  "Red"; a synthetic JPEG is described correctly.
- `cargo test -p opencode-provider --no-default-features image_file_part_is_sent_as_image_url`
  passes.
- Note: the `/session/{id}/stream` endpoint used by `run --attach` still sends
  `tools: None, system: None`; the full agentic path is `/session/{id}/prompt`.
  That is a separate gap tracked here for visibility, not changed by this fix.

## Acceptance Criteria

- [x] File/image user parts reach the provider request.
- [x] OpenAI-compatible chat serializes image file parts as `image_url`.
- [x] Regression test added and passing.
- [ ] Shipped binary rebuilt/installed with the fix (owner-gated release).
- [ ] `run --attach` (`/stream`) agentic/tool parity (separate follow-up).
