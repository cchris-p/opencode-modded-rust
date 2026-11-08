//! Map-backed code search with a bounded grep fallback (`SCOPE-019`).
//!
//! The tool only sees the neutral `WorkspaceSearch` capability injected through
//! [`ToolContext`]. It never parses model strings or calls FFI. When the
//! capability is absent, the map is stale/unavailable, or the map returns no
//! matches, the tool falls back to the same bounded walk as `grep`.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use opencode_types::workspace::{MapFreshness, WorkspaceSearchRequest, WorkspaceSearchResponse};

use crate::{
    external_directory_permission_request, Metadata, PermissionRequest, Tool, ToolContext,
    ToolError, ToolResult,
};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

#[derive(Debug, Deserialize)]
struct CodeSearchParams {
    query: String,
    #[serde(default)]
    path: Option<String>,
    /// `code` (map path), `prose` (grep only), or `auto` (default).
    #[serde(default)]
    mode: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchMode {
    Code,
    Prose,
    Auto,
}

impl SearchMode {
    fn parse(value: Option<&str>) -> Result<Self, ToolError> {
        match value.unwrap_or("auto") {
            "code" => Ok(Self::Code),
            "prose" => Ok(Self::Prose),
            "auto" => Ok(Self::Auto),
            other => Err(ToolError::InvalidArguments(format!(
                "mode must be code, prose, or auto (got {other})"
            ))),
        }
    }
}

pub struct WorkspaceCodeSearchTool;

#[async_trait]
impl Tool for WorkspaceCodeSearchTool {
    fn id(&self) -> &str {
        "code_search"
    }

    fn description(&self) -> &str {
        "Locate code across the workspace by symbol or text in one call. Uses the persistent ScopeMux map when available and falls back to bounded grep. Use mode=\"prose\" for markdown/docs, which always uses grep."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Symbol, identifier, or text to locate (literal match)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (defaults to the workspace)"
                },
                "mode": {
                    "type": "string",
                    "enum": ["code", "prose", "auto"],
                    "default": "auto",
                    "description": "code: map-backed with grep fallback; prose: grep only; auto: map when available"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 200,
                    "default": 50,
                    "description": "Maximum results to return"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: CodeSearchParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid parameters: {e}")))?;
        let query = params.query.trim().to_string();
        if query.is_empty() {
            return Err(ToolError::InvalidArguments(
                "query must not be empty".to_string(),
            ));
        }
        let mode = SearchMode::parse(params.mode.as_deref())?;
        let limit = params.limit.clamp(1, MAX_LIMIT);

        let base_dir = params
            .path
            .clone()
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| ctx.directory.clone());
        let base_dir = PathBuf::from(&base_dir);
        let base_dir_str = base_dir.to_string_lossy().to_string();

        if ctx.is_external_path(&base_dir_str) {
            let mut request = external_directory_permission_request(
                &base_dir_str,
                crate::ExternalDirectoryKind::Directory,
            );
            request
                .metadata
                .insert("path".to_string(), serde_json::json!(&base_dir_str));
            ctx.ask_permission(request).await?;
        }

        ctx.ask_permission(
            PermissionRequest::new("code_search")
                .with_pattern(&query)
                .always_allow()
                .with_metadata("path", serde_json::json!(&base_dir_str)),
        )
        .await?;

        let workspace_root = if ctx.worktree.is_empty() {
            ctx.directory.clone()
        } else {
            ctx.worktree.clone()
        };

        let map_response = if mode != SearchMode::Prose {
            ctx.workspace_search.as_ref().map(|capability| {
                capability.search_code(&WorkspaceSearchRequest::new(
                    workspace_root.clone(),
                    query.clone(),
                    limit,
                ))
            })
        } else {
            None
        };

        let map_usable = map_response.as_ref().is_some_and(|response| {
            !response.candidates.is_empty()
                && !matches!(
                    response.freshness,
                    MapFreshness::Stale | MapFreshness::Unknown
                )
        });

        if map_usable {
            let response = map_response.expect("checked above");
            return Ok(map_result(&query, response, limit));
        }

        let fallback_reason = fallback_reason(mode, map_response.as_ref());
        let scan_base = base_dir.clone();
        let scan_query = query.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            crate::grep_tool::scan_literal(scan_base, &scan_query, false)
        })
        .await
        .map_err(|e| ToolError::ExecutionError(format!("code_search task failed: {e}")))?;

        Ok(grep_result(&query, outcome, &fallback_reason, limit))
    }
}

fn fallback_reason(mode: SearchMode, response: Option<&WorkspaceSearchResponse>) -> String {
    if mode == SearchMode::Prose {
        return "prose_mode".to_string();
    }
    match response {
        None => "map_unavailable".to_string(),
        Some(response) => match response.freshness {
            MapFreshness::Stale => "map_stale".to_string(),
            MapFreshness::Unknown => "map_unknown".to_string(),
            _ => response
                .fallback_reason
                .map(|reason| format!("{reason:?}").to_lowercase())
                .unwrap_or_else(|| "no_matches".to_string()),
        },
    }
}

fn map_result(query: &str, response: WorkspaceSearchResponse, limit: usize) -> ToolResult {
    let total = response.candidates.len();
    let shown: Vec<_> = response.candidates.iter().take(limit).collect();
    let mut lines = vec![format!(
        "Found {} code matches (backend: scopemux, freshness: {:?})",
        total, response.freshness
    )];
    for candidate in &shown {
        let symbol = candidate.symbol.as_deref().unwrap_or("<file>");
        let block_id = candidate.block_id.as_deref().unwrap_or("-");
        lines.push(format!(
            "- {} :: {} [block: {}] ({:?}) - {}",
            candidate.path, symbol, block_id, candidate.confidence, candidate.provenance
        ));
    }
    if total > limit {
        lines.push(format!(
            "(Results truncated: showing {} of {} matches.)",
            limit, total
        ));
    }

    let mut metadata = Metadata::new();
    metadata.insert("backend".into(), serde_json::json!("scopemux"));
    metadata.insert(
        "freshness".into(),
        serde_json::json!(format!("{:?}", response.freshness).to_lowercase()),
    );
    metadata.insert("matches".into(), serde_json::json!(total));
    metadata.insert("truncated".into(), serde_json::json!(total > limit));

    ToolResult {
        title: format!("code_search '{query}'"),
        output: lines.join("\n"),
        metadata,
        truncated: total > limit,
    }
}

fn grep_result(
    query: &str,
    outcome: crate::grep_tool::ScanOutcome,
    fallback_reason: &str,
    limit: usize,
) -> ToolResult {
    let total = outcome.matches.len();
    let mut lines = vec![format!(
        "Found {} matches (backend: grep, fallback: {})",
        total, fallback_reason
    )];

    if total == 0 {
        lines.push(format!("No matches found for '{}'", query));
    } else {
        let mut current_file = "";
        for matched in outcome.matches.iter().take(limit) {
            if current_file != matched.path {
                if !current_file.is_empty() {
                    lines.push(String::new());
                }
                current_file = &matched.path;
                lines.push(format!("{}:", matched.path));
            }
            lines.push(format!(
                "  Line {}: {}",
                matched.line_num, matched.line_text
            ));
        }
        if total > limit {
            lines.push(format!(
                "(Results truncated: showing {} of {} matches.)",
                limit, total
            ));
        }
    }

    if outcome.budget_exhausted {
        lines.push(format!(
            "(Search stopped after scanning {} files to stay responsive.)",
            outcome.files_scanned
        ));
    }
    if outcome.has_errors {
        lines.push("(Some paths were inaccessible and skipped)".to_string());
    }

    let truncated = total > limit || outcome.budget_exhausted;
    let mut metadata = Metadata::new();
    metadata.insert("backend".into(), serde_json::json!("grep"));
    metadata.insert("fallbackReason".into(), serde_json::json!(fallback_reason));
    metadata.insert("matches".into(), serde_json::json!(total));
    metadata.insert("truncated".into(), serde_json::json!(truncated));
    metadata.insert(
        "filesScanned".into(),
        serde_json::json!(outcome.files_scanned),
    );

    ToolResult {
        title: format!("code_search '{query}'"),
        output: lines.join("\n"),
        metadata,
        truncated,
    }
}

/// True when the query would be searched by the code map rather than prose.
pub fn is_code_search_mode(mode: &str) -> bool {
    mode != "prose"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use opencode_types::{MapFreshness, RetrievalFallbackReason};

    struct StubSearch {
        response: WorkspaceSearchResponse,
    }

    impl opencode_types::workspace::WorkspaceSearch for StubSearch {
        fn search_code(&self, _request: &WorkspaceSearchRequest) -> WorkspaceSearchResponse {
            self.response.clone()
        }
    }

    fn ctx(dir: &std::path::Path) -> ToolContext {
        ToolContext::new(
            "session".into(),
            "message".into(),
            dir.to_string_lossy().to_string(),
        )
    }

    #[tokio::test]
    async fn map_backed_result_carries_block_id_and_freshness() {
        let dir = tempfile::tempdir().unwrap();
        let response = WorkspaceSearchResponse {
            candidates: vec![opencode_types::RetrievalCandidate {
                kind: opencode_types::RetrievalCandidateKind::Symbol,
                path: "src/lib.rs".to_string(),
                symbol: Some("helper".to_string()),
                block_id: Some("sym:helper".to_string()),
                snippet: None,
                provenance: "scopemux: search".to_string(),
                confidence: opencode_types::RetrievalConfidence::High,
                score: 1.0,
                estimated_tokens: Some(4),
                origin: Some(opencode_types::RetrievalOrigin::Parsed),
                lifecycle: None,
            }],
            freshness: MapFreshness::Fresh,
            fallback_reason: None,
        };
        let capability: Arc<dyn opencode_types::workspace::WorkspaceSearch> =
            Arc::new(StubSearch { response });
        let ctx = ctx(dir.path()).with_workspace_search(Some(capability));

        let result = WorkspaceCodeSearchTool
            .execute(serde_json::json!({"query": "helper"}), ctx)
            .await
            .unwrap();

        assert_eq!(result.metadata["backend"], serde_json::json!("scopemux"));
        assert!(result.output.contains("sym:helper"));
        assert!(result.output.contains("fresh"));
    }

    #[tokio::test]
    async fn absent_capability_falls_back_to_bounded_grep() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "pub fn needle() {}\n").unwrap();

        let result = WorkspaceCodeSearchTool
            .execute(
                serde_json::json!({"query": "needle", "mode": "code"}),
                ctx(dir.path()),
            )
            .await
            .unwrap();

        assert_eq!(result.metadata["backend"], serde_json::json!("grep"));
        assert_eq!(
            result.metadata["fallbackReason"],
            serde_json::json!("map_unavailable")
        );
        assert!(result.output.contains("needle"));
    }

    #[tokio::test]
    async fn stale_map_falls_back_to_grep() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "pub fn needle() {}\n").unwrap();
        let response = WorkspaceSearchResponse {
            candidates: Vec::new(),
            freshness: MapFreshness::Stale,
            fallback_reason: Some(RetrievalFallbackReason::ProviderUnavailable),
        };
        let capability: Arc<dyn opencode_types::workspace::WorkspaceSearch> =
            Arc::new(StubSearch { response });
        let ctx = ctx(dir.path()).with_workspace_search(Some(capability));

        let result = WorkspaceCodeSearchTool
            .execute(serde_json::json!({"query": "needle"}), ctx)
            .await
            .unwrap();

        assert_eq!(result.metadata["backend"], serde_json::json!("grep"));
        assert_eq!(
            result.metadata["fallbackReason"],
            serde_json::json!("map_stale")
        );
    }

    #[tokio::test]
    async fn prose_mode_never_uses_the_map() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "prose needle here\n").unwrap();
        let response = WorkspaceSearchResponse {
            candidates: Vec::new(),
            freshness: MapFreshness::Fresh,
            fallback_reason: None,
        };
        let capability: Arc<dyn opencode_types::workspace::WorkspaceSearch> =
            Arc::new(StubSearch { response });
        let ctx = ctx(dir.path()).with_workspace_search(Some(capability));

        let result = WorkspaceCodeSearchTool
            .execute(
                serde_json::json!({"query": "prose needle", "mode": "prose"}),
                ctx,
            )
            .await
            .unwrap();

        assert_eq!(result.metadata["backend"], serde_json::json!("grep"));
        assert_eq!(
            result.metadata["fallbackReason"],
            serde_json::json!("prose_mode")
        );
    }
}
