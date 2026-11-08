//! Shared workspace-map contracts and policy (`SCOPE-018`).
//!
//! These types and helpers are the single source of truth for vocabulary and
//! decisions shared by the map engine, the supervisor, the retrieval provider,
//! the tool surface, and the file watcher:
//!
//! - the ignore/exclusion policy consumed by grep, the watcher, and indexing;
//! - the code/prose predicate (one extension table, shared with `SCOPE-010`);
//! - the map freshness vocabulary and measurement report;
//! - the neutral [`WorkspaceSearch`] capability the runtime injects into
//!   `ToolContext` so the tool never parses model strings or touches FFI;
//! - path normalization and token-budget helpers.
//!
//! DRY applies to decisions and vocabulary, not to forcing grep and map search
//! to share an algorithm.

use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::{RetrievalCandidate, RetrievalFallbackReason};

/// A source language `scopemux-core` can parse.
///
/// This is the only extension table for code classification. The provider maps
/// it to the core `Language` enum through [`SourceLanguage::scopemux_language_id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLanguage {
    C,
    Cpp,
    Python,
    JavaScript,
    TypeScript,
    Rust,
}

impl SourceLanguage {
    /// Classify a path by extension, ignoring case.
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => Some(Self::Cpp),
            "py" => Some(Self::Python),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "rs" => Some(Self::Rust),
            _ => None,
        }
    }

    /// Stable `scopemux-core` `Language` enum id for this language.
    pub fn scopemux_language_id(self) -> i32 {
        match self {
            Self::C => 1,
            Self::Cpp => 2,
            Self::Python => 3,
            Self::JavaScript => 4,
            Self::TypeScript => 5,
            Self::Rust => 6,
        }
    }
}

/// True when the path has a language grammar the map can parse.
pub fn is_code_path(path: &str) -> bool {
    SourceLanguage::from_path(path).is_some()
}

/// True when the path is a known prose/document format.
///
/// Prose is searched with the generic/grep path, never with a code grammar.
pub fn is_prose_path(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "md" | "markdown" | "mdx" | "txt" | "rst" | "adoc" | "asciidoc" | "org"
    )
}

/// Build and dependency directories that are never worth scanning or indexing.
///
/// One list consumed by grep (`opencode-tool`), the watcher
/// (`opencode-watcher`), and the map indexer/supervisor.
pub const EXCLUDED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".opencode",
    "dist",
    "build",
    ".next",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
];

/// True when a directory name is on the shared exclusion list.
pub fn is_excluded_dir(name: &str) -> bool {
    EXCLUDED_DIRS
        .iter()
        .any(|excluded| excluded.eq_ignore_ascii_case(name))
}

/// True when any component of the path is a shared excluded directory.
pub fn path_has_excluded_dir(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => name.to_str().is_some_and(is_excluded_dir),
        _ => false,
    })
}

/// Glob patterns matching the shared excluded directories.
///
/// The watcher consumes these so it never emits events for build/vendor trees.
pub fn default_ignore_globs() -> Vec<String> {
    EXCLUDED_DIRS
        .iter()
        .map(|dir| format!("**/{dir}/**"))
        .collect()
}

/// Transient editor/OS files the watcher also ignores.
pub fn default_ignore_file_globs() -> &'static [&'static str] {
    &["**/.DS_Store", "**/*.swp", "**/*.swo", "**/*~"]
}

/// Map freshness vocabulary shared by measurement, provenance, and policy.
///
/// Measurement fills this in; the runtime decides what to do with it. No
/// fallback policy lives on the type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapFreshness {
    /// The map reflects the latest observed revision.
    Fresh,
    /// The map is behind but still usable with a warning.
    Lagging,
    /// The map may anchor a task to code that no longer exists.
    Stale,
    /// No measurement is available.
    #[default]
    Unknown,
}

/// Result of re-anchoring a projected plan node after an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReAnchorResult {
    /// The anchor still resolves to the same node.
    Anchored,
    /// The anchor moved but still resolves to a node.
    Shifted,
    /// The anchor no longer resolves.
    Missing,
}

/// Measurement the supervisor/engine fills for `SCOPE-009`.
///
/// The report records facts only. The runtime owns the fresh/lagging/stale/
/// unknown response (use, warn, or generic fallback).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapFreshnessReport {
    pub state: MapFreshness,
    /// Last revision/build counter observed for the workspace map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_revision: Option<u64>,
    /// Number of paths currently queued for incremental update.
    #[serde(default)]
    pub dirty_set_size: usize,
    /// Delay between the file event and the applied update, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watcher_lag_ms: Option<u64>,
    /// Duration of the last incremental rebuild, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_rebuild_duration_ms: Option<u64>,
    /// Re-anchor result for a projected plan node, when one was checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub re_anchor: Option<ReAnchorResult>,
}

impl Default for MapFreshnessReport {
    fn default() -> Self {
        Self {
            state: MapFreshness::Unknown,
            last_revision: None,
            dirty_set_size: 0,
            watcher_lag_ms: None,
            last_rebuild_duration_ms: None,
            re_anchor: None,
        }
    }
}

/// One map-backed code-search request.
///
/// The tool builds this from user arguments; the supervisor answers it from the
/// persistent map. The result uses the shared [`RetrievalCandidate`] evidence
/// type so the tool does not define a competing result shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSearchRequest {
    /// Canonical workspace root the search should run against.
    pub workspace_root: String,
    pub query: String,
    pub max_results: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<SourceLanguage>,
}

impl WorkspaceSearchRequest {
    pub fn new(
        workspace_root: impl Into<String>,
        query: impl Into<String>,
        max_results: usize,
    ) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            query: query.into(),
            max_results,
            language: None,
        }
    }
}

/// The answer to a [`WorkspaceSearchRequest`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSearchResponse {
    pub candidates: Vec<RetrievalCandidate>,
    pub freshness: MapFreshness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<RetrievalFallbackReason>,
}

impl WorkspaceSearchResponse {
    pub fn empty(freshness: MapFreshness) -> Self {
        Self {
            candidates: Vec::new(),
            freshness,
            fallback_reason: None,
        }
    }
}

/// Neutral capability the runtime injects into `ToolContext`.
///
/// The tool depends on this trait, never on FFI or model strings. The runtime
/// decides activation once and installs the capability only on the map-backed
/// path; absent capability means grep/generic search.
pub trait WorkspaceSearch: Send + Sync {
    fn search_code(&self, request: &WorkspaceSearchRequest) -> WorkspaceSearchResponse;
}

/// Strip a `file://` scheme, if present.
pub fn strip_file_scheme(path: &str) -> &str {
    path.strip_prefix("file://").unwrap_or(path)
}

/// Normalize a workspace path to one comparable string.
///
/// Removes a `file://` scheme and trailing separators. It does not resolve
/// symlinks or require the path to exist.
pub fn normalize_path(path: &str) -> String {
    let path = strip_file_scheme(path);
    let trimmed = path.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        path.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Resolve a possibly relative path against a workspace root.
pub fn resolve_workspace_path(root: &str, path: &str) -> String {
    let path = strip_file_scheme(path);
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return normalize_path(path);
    }
    let root = normalize_path(root);
    normalize_path(&format!("{root}/{path}"))
}

/// Character-based token estimate shared by prompt and tool output.
///
/// Matches the runtime's existing `estimate_tokens` heuristic (four characters
/// per token) so callers do not each invent a budget.
pub fn estimate_tokens(text: &str) -> u64 {
    text.chars().count() as u64 / 4
}

/// Truncate `text` to at most `max_tokens`, on a character boundary.
///
/// Returns the original text when it already fits. Truncated text gets an
/// ellipsis so the caller can tell the budget was hit.
pub fn truncate_to_token_budget(text: &str, max_tokens: u64) -> String {
    if max_tokens == 0 {
        return String::new();
    }
    if estimate_tokens(text) <= max_tokens {
        return text.to_string();
    }
    let max_chars = usize::try_from(max_tokens.saturating_mul(4)).unwrap_or(usize::MAX);
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_extension_table_classifies_code_and_prose() {
        assert_eq!(SourceLanguage::from_path("a.c"), Some(SourceLanguage::C));
        assert_eq!(
            SourceLanguage::from_path("a.cpp"),
            Some(SourceLanguage::Cpp)
        );
        assert_eq!(
            SourceLanguage::from_path("a.py"),
            Some(SourceLanguage::Python)
        );
        assert_eq!(
            SourceLanguage::from_path("a.js"),
            Some(SourceLanguage::JavaScript)
        );
        assert_eq!(
            SourceLanguage::from_path("a.ts"),
            Some(SourceLanguage::TypeScript)
        );
        assert_eq!(
            SourceLanguage::from_path("a.rs"),
            Some(SourceLanguage::Rust)
        );
        assert_eq!(SourceLanguage::from_path("a.txt"), None);
        assert!(is_code_path("src/lib.rs"));
        assert!(!is_code_path("README.md"));
        assert!(is_prose_path("README.md"));
        assert!(is_prose_path("docs/notes.txt"));
        assert!(!is_prose_path("src/lib.rs"));
        assert_eq!(SourceLanguage::Rust.scopemux_language_id(), 6);
    }

    #[test]
    fn excluded_dirs_are_the_single_ignore_source() {
        for dir in [
            "target",
            "node_modules",
            ".git",
            ".opencode",
            "dist",
            "build",
            ".venv",
            "venv",
            "__pycache__",
            ".cache",
        ] {
            assert!(is_excluded_dir(dir), "{dir} should be excluded");
        }
        assert!(is_excluded_dir("TARGET"));
        assert!(!is_excluded_dir("src"));

        let globs = default_ignore_globs();
        assert!(globs.contains(&"**/target/**".to_string()));
        assert!(globs.contains(&"**/node_modules/**".to_string()));
        assert!(globs.contains(&"**/.git/**".to_string()));

        assert!(path_has_excluded_dir(Path::new("/repo/target/debug/x.rs")));
        assert!(path_has_excluded_dir(Path::new("node_modules/pkg/a.js")));
        assert!(!path_has_excluded_dir(Path::new("src/targets/a.rs")));
    }

    #[test]
    fn freshness_and_reanchor_vocabularies_serialize() {
        assert_eq!(
            serde_json::to_string(&MapFreshness::Fresh).unwrap(),
            "\"fresh\""
        );
        assert_eq!(
            serde_json::to_string(&MapFreshness::Unknown).unwrap(),
            "\"unknown\""
        );
        assert_eq!(
            serde_json::to_string(&ReAnchorResult::Shifted).unwrap(),
            "\"shifted\""
        );

        let report = MapFreshnessReport {
            state: MapFreshness::Lagging,
            last_revision: Some(42),
            dirty_set_size: 3,
            watcher_lag_ms: Some(120),
            last_rebuild_duration_ms: Some(8),
            re_anchor: Some(ReAnchorResult::Anchored),
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: MapFreshnessReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, report);
    }

    #[test]
    fn path_normalization_handles_scheme_and_relative_paths() {
        assert_eq!(strip_file_scheme("file:///tmp/a.rs"), "/tmp/a.rs");
        assert_eq!(normalize_path("/repo/src/lib.rs/"), "/repo/src/lib.rs");
        assert_eq!(
            resolve_workspace_path("/repo/", "src/lib.rs"),
            "/repo/src/lib.rs"
        );
        assert_eq!(
            resolve_workspace_path("/repo", "file:///abs/lib.rs"),
            "/abs/lib.rs"
        );
    }

    #[test]
    fn token_budget_helper_truncates_on_boundaries() {
        assert_eq!(estimate_tokens("12345678"), 2);
        let short = truncate_to_token_budget("short", 4);
        assert_eq!(short, "short");
        let long = truncate_to_token_budget("abcdefghij", 1);
        assert_eq!(long, "abcd...");
        assert_eq!(truncate_to_token_budget("abcdefghij", 0), "");
    }
}
