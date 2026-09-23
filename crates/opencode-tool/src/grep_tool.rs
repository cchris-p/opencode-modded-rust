use async_trait::async_trait;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use walkdir::WalkDir;

use crate::{
    external_directory_permission_request, Metadata, Tool, ToolContext, ToolError, ToolResult,
};

const MAX_LINE_LENGTH: usize = 2000;
const RESULT_LIMIT: usize = 100;
const MAX_STORED_MATCHES: usize = 5_000;
const MAX_FILES_SCANNED: usize = 20_000;
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SCAN_DURATION: Duration = Duration::from_secs(15);

/// Build and dependency directories that are never worth scanning. Without
/// these exclusions a repo-root search walks build artifacts (for example a
/// Rust `target/` directory with hundreds of thousands of files) and stalls the
/// whole session.
const EXCLUDED_DIRS: &[&str] = &[
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

pub struct GrepTool {
    directory: PathBuf,
}

impl GrepTool {
    pub fn new() -> Self {
        Self {
            directory: std::env::current_dir().unwrap_or_default(),
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

struct GrepMatch {
    path: String,
    mtime: SystemTime,
    line_num: usize,
    line_text: String,
}

struct ScanOutcome {
    matches: Vec<GrepMatch>,
    has_errors: bool,
    files_scanned: usize,
    budget_exhausted: bool,
}

fn is_excluded_dir(name: &str) -> bool {
    EXCLUDED_DIRS
        .iter()
        .any(|excluded| excluded.eq_ignore_ascii_case(name))
}

fn truncate_line(line: &str) -> String {
    if line.len() <= MAX_LINE_LENGTH {
        return line.to_string();
    }

    let mut end = MAX_LINE_LENGTH;
    while end > 0 && !line.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &line[..end])
}

/// Walks and scans the tree synchronously. Callers must run this on a blocking
/// thread (`spawn_blocking`); doing this work on an async runtime thread starves
/// the runtime and hangs the server and TUI.
fn scan_blocking(
    base_dir: PathBuf,
    regex: Regex,
    glob_pattern: Option<glob::Pattern>,
    include_hidden: bool,
) -> ScanOutcome {
    let mut matches: Vec<GrepMatch> = Vec::new();
    let mut has_errors = false;
    let mut files_scanned = 0usize;
    let mut budget_exhausted = false;
    let deadline = Instant::now() + MAX_SCAN_DURATION;

    let walker = WalkDir::new(&base_dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(move |entry| {
            if entry.depth() == 0 {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            if entry.file_type().is_dir() && is_excluded_dir(&name) {
                return false;
            }
            include_hidden || !name.starts_with('.')
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if files_scanned >= MAX_FILES_SCANNED || Instant::now() >= deadline {
            budget_exhausted = true;
            break;
        }

        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        files_scanned += 1;

        if let Some(ref gp) = glob_pattern {
            let rel_path = path.strip_prefix(&base_dir).unwrap_or(path);
            if !gp.matches(&rel_path.to_string_lossy()) {
                continue;
            }
        }

        let mtime = path
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        match File::open(path) {
            Ok(file) => {
                let path_str = path.to_string_lossy().to_string();
                let reader = BufReader::new(file).take(MAX_FILE_BYTES);

                for (line_num, line_result) in reader.lines().enumerate() {
                    let Ok(line) = line_result else {
                        continue;
                    };
                    if regex.is_match(&line) {
                        matches.push(GrepMatch {
                            path: path_str.clone(),
                            mtime,
                            line_num: line_num + 1,
                            line_text: truncate_line(&line),
                        });

                        if matches.len() >= MAX_STORED_MATCHES {
                            budget_exhausted = true;
                            break;
                        }
                    }
                }

                if budget_exhausted {
                    break;
                }
            }
            Err(_) => has_errors = true,
        }
    }

    matches.sort_by(|a, b| b.mtime.cmp(&a.mtime));

    ScanOutcome {
        matches,
        has_errors,
        files_scanned,
        budget_exhausted,
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn id(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Fast content search tool. Searches file contents using regular expressions. Results sorted by file modification time (most recent first)."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in"
                },
                "glob": {
                    "type": "string",
                    "description": "File pattern to include (e.g., '*.js')"
                },
                "ignore_case": {
                    "type": "boolean",
                    "description": "Case insensitive search"
                },
                "hidden": {
                    "type": "boolean",
                    "description": "Search hidden files and directories (default: false)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let pattern: String = args["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("pattern is required".into()))?
            .to_string();

        let search_path: String = args["path"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| ctx.directory.clone());

        let glob_filter: Option<String> = args["glob"].as_str().map(|s| s.to_string());

        let ignore_case: bool = args["ignore_case"].as_bool().unwrap_or(false);

        let include_hidden: bool = args["hidden"].as_bool().unwrap_or(false);

        let base_dir = if search_path.is_empty() {
            &self.directory
        } else {
            Path::new(&search_path)
        };

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
            crate::PermissionRequest::new("grep")
                .with_pattern(&pattern)
                .always_allow()
                .with_metadata("path", serde_json::json!(&base_dir_str)),
        )
        .await?;

        let regex_pattern = if ignore_case {
            format!("(?i){}", pattern)
        } else {
            pattern.clone()
        };

        let regex = Regex::new(&regex_pattern)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid regex: {}", e)))?;

        let glob_pattern = glob_filter
            .as_ref()
            .and_then(|g| glob::Pattern::new(g).ok());

        let scan_base = base_dir.to_path_buf();
        let outcome = tokio::task::spawn_blocking(move || {
            scan_blocking(scan_base, regex, glob_pattern, include_hidden)
        })
        .await
        .map_err(|e| ToolError::ExecutionError(format!("grep task failed: {}", e)))?;

        let ScanOutcome {
            matches,
            has_errors,
            files_scanned,
            budget_exhausted,
        } = outcome;

        let limit = RESULT_LIMIT;
        let total_matches = matches.len();
        let truncated = total_matches > limit || budget_exhausted;

        let title = format!("grep '{}'", pattern);
        let output = if total_matches == 0 && !budget_exhausted {
            format!("No matches found for pattern '{}'", pattern)
        } else {
            let mut output_lines = vec![format!(
                "Found {} matches{}",
                total_matches,
                if total_matches > limit {
                    format!(" (showing first {})", limit)
                } else {
                    String::new()
                }
            )];

            let display_matches: Vec<&GrepMatch> = matches.iter().take(limit).collect();
            let mut current_file = "";

            for m in display_matches {
                if current_file != m.path {
                    if !current_file.is_empty() {
                        output_lines.push(String::new());
                    }
                    current_file = &m.path;
                    output_lines.push(format!("{}:", m.path));
                }
                output_lines.push(format!("  Line {}: {}", m.line_num, m.line_text));
            }

            if total_matches > limit {
                output_lines.push(String::new());
                output_lines.push(format!(
                    "(Results truncated: showing {} of {} matches ({} hidden). Consider using a more specific path or pattern.)",
                    limit, total_matches, total_matches - limit
                ));
            }

            if budget_exhausted {
                output_lines.push(String::new());
                output_lines.push(format!(
                    "(Search stopped after scanning {} files to stay responsive; results may be incomplete. Narrow the path or pattern.)",
                    files_scanned
                ));
            }

            if has_errors {
                output_lines.push(String::new());
                output_lines.push("(Some paths were inaccessible and skipped)".to_string());
            }

            output_lines.join("\n")
        };

        Ok(ToolResult {
            title,
            output,
            metadata: {
                let mut m = Metadata::new();
                m.insert("matches".into(), serde_json::json!(total_matches));
                m.insert("truncated".into(), serde_json::json!(truncated));
                m.insert("hasErrors".into(), serde_json::json!(has_errors));
                m.insert("filesScanned".into(), serde_json::json!(files_scanned));
                m.insert(
                    "budgetExhausted".into(),
                    serde_json::json!(budget_exhausted),
                );
                m
            },
            truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_for(dir: &Path) -> ToolContext {
        ToolContext::new(
            "session-test".into(),
            "message-test".into(),
            dir.to_string_lossy().to_string(),
        )
    }

    async fn run_grep(dir: &Path, pattern: &str, extra: serde_json::Value) -> ToolResult {
        let tool = GrepTool::new();
        let mut args = serde_json::json!({
            "pattern": pattern,
            "path": dir.to_string_lossy(),
        });
        if let (Some(map), Some(extra_map)) = (args.as_object_mut(), extra.as_object()) {
            for (key, value) in extra_map {
                map.insert(key.clone(), value.clone());
            }
        }
        tool.execute(args, ctx_for(dir))
            .await
            .expect("grep should succeed")
    }

    #[tokio::test]
    async fn skips_build_and_vendor_directories() {
        // Regression: a repo-root grep walked `target/` (hundreds of thousands
        // of files) on the async runtime thread and stalled the whole session.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("target/debug")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        std::fs::write(root.join("src/hit.txt"), "NEEDLE\n").unwrap();
        std::fs::write(root.join("target/debug/hit.txt"), "NEEDLE\n").unwrap();
        std::fs::write(root.join("node_modules/pkg/hit.txt"), "NEEDLE\n").unwrap();

        let result = run_grep(root, "NEEDLE", serde_json::json!({})).await;

        assert_eq!(
            result.metadata.get("matches").and_then(|v| v.as_u64()),
            Some(1),
            "only the source match should remain:\n{}",
            result.output
        );
        assert!(result.output.contains("src"));
        assert!(!result.output.contains("node_modules"));
        assert!(!result.output.contains("target/debug"));
    }

    #[tokio::test]
    async fn reports_no_matches() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "hello\n").unwrap();

        let result = run_grep(tmp.path(), "absent-token", serde_json::json!({})).await;

        assert!(result.output.contains("No matches found"));
    }
}
