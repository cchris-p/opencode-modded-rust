use async_trait::async_trait;
use glob::Pattern;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::{Metadata, PermissionRequest, Tool, ToolContext, ToolError, ToolResult};

const IGNORE_PATTERNS: &[&str] = &[
    "node_modules/",
    "__pycache__/",
    ".git/",
    "dist/",
    "build/",
    "target/",
    "vendor/",
    "bin/",
    "obj/",
    ".idea/",
    ".vscode/",
    ".zig-cache/",
    "zig-out",
    ".coverage",
    "coverage/",
    "vendor/",
    "tmp/",
    "temp/",
    ".cache/",
    "cache/",
    "logs/",
    ".venv/",
    "venv/",
    "env/",
];

const LIMIT: usize = 100;

pub struct LsTool {}

impl LsTool {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for LsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Deserialize)]
struct LsInput {
    path: Option<String>,
    ignore: Option<Vec<String>>,
}

fn has_glob_meta(pattern: &str) -> bool {
    pattern
        .chars()
        .any(|ch| matches!(ch, '*' | '?' | '[' | ']' | '{' | '}'))
}

#[async_trait]
impl Tool for LsTool {
    fn id(&self) -> &str {
        "ls"
    }

    fn description(&self) -> &str {
        "Lists the immediate files and directories inside a given path (one level, not recursive)."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the directory to list (must be absolute, not relative)"
                },
                "ignore": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of glob patterns to ignore"
                }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let input: LsInput = serde_json::from_value(args).unwrap_or(LsInput {
            path: None,
            ignore: None,
        });

        let requested_path = input.path.unwrap_or_else(|| ".".to_string());
        let mut base_dir = if Path::new(&requested_path).is_absolute() {
            PathBuf::from(&requested_path)
        } else {
            PathBuf::from(&ctx.directory).join(&requested_path)
        };
        if let Ok(canonical) = base_dir.canonicalize() {
            base_dir = canonical;
        }
        let base_dir_str = base_dir.to_string_lossy().to_string();

        ctx.ask_permission(
            PermissionRequest::new("list")
                .with_pattern(&base_dir_str)
                .with_metadata("path", serde_json::json!(&base_dir_str))
                .always_allow(),
        )
        .await?;

        if !base_dir.exists() {
            return Err(ToolError::FileNotFound(base_dir.display().to_string()));
        }

        if !base_dir.is_dir() {
            return Err(ToolError::ExecutionError(format!(
                "{} is not a directory",
                base_dir.display()
            )));
        }

        let mut ignore_set: HashSet<String> = IGNORE_PATTERNS
            .iter()
            .map(|s| s.trim_end_matches('/').to_string())
            .collect();
        let mut ignore_globs: Vec<Pattern> = Vec::new();

        if let Some(custom_ignore) = input.ignore {
            for pattern in custom_ignore {
                let normalized = pattern.trim_start_matches('!').trim();
                if normalized.is_empty() {
                    continue;
                }

                if has_glob_meta(normalized) {
                    if let Ok(glob) = Pattern::new(normalized) {
                        ignore_globs.push(glob);
                    }
                } else {
                    ignore_set.insert(normalized.trim_end_matches('/').to_string());
                }
            }
        }

        // Bounded top-level listing: list the immediate children of the
        // requested directory only. Directories are always included (even when
        // they contain no files); files may be capped per listing, but the cap
        // must never hide the directory's own children. BUG-007.
        let mut entries = tokio::fs::read_dir(&base_dir).await.map_err(|e| {
            ToolError::ExecutionError(format!(
                "Failed to read directory {}: {}",
                base_dir.display(),
                e
            ))
        })?;

        let mut dirs: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            ToolError::ExecutionError(format!(
                "Failed to read directory {}: {}",
                base_dir.display(),
                e
            ))
        })? {
            let name = entry.file_name().to_string_lossy().replace('\\', "/");
            if name.is_empty() {
                continue;
            }

            let should_skip =
                ignore_set.contains(&name) || ignore_globs.iter().any(|glob| glob.matches(&name));
            if should_skip {
                continue;
            }

            let file_type = entry.file_type().await.map_err(|e| {
                ToolError::ExecutionError(format!(
                    "Failed to stat {}: {}",
                    entry.path().display(),
                    e
                ))
            })?;

            if file_type.is_dir() {
                dirs.push(format!("{}/", name));
            } else {
                files.push(name);
            }
        }

        dirs.sort();
        files.sort();

        let total_files = files.len();
        let truncated = total_files > LIMIT;
        if truncated {
            files.truncate(LIMIT);
        }

        let mut output = format!("{}/\n", base_dir.display());
        for dir in &dirs {
            output.push_str(&format!("  {}\n", dir));
        }
        for file in &files {
            output.push_str(&format!("  {}\n", file));
        }
        if truncated {
            output.push_str(&format!(
                "\n... ({} of {} files shown; {} more not listed)\n",
                files.len(),
                total_files,
                total_files - files.len()
            ));
        }

        let title = match base_dir.strip_prefix(Path::new(&ctx.worktree)) {
            Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
            Ok(rel) => rel.to_string_lossy().to_string(),
            Err(_) => base_dir.display().to_string(),
        };

        Ok(ToolResult {
            title,
            output,
            metadata: {
                let mut m = Metadata::new();
                m.insert("dirs".into(), serde_json::json!(dirs.len()));
                m.insert("files".into(), serde_json::json!(files.len()));
                m.insert("total_files".into(), serde_json::json!(total_files));
                m.insert("truncated".into(), serde_json::json!(truncated));
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

    async fn run_ls(dir: &Path) -> ToolResult {
        let tool = LsTool::new();
        tool.execute(
            serde_json::json!({ "path": dir.to_string_lossy() }),
            ctx_for(dir),
        )
        .await
        .expect("ls should succeed")
    }

    #[tokio::test]
    async fn lists_all_top_level_directories_even_with_many_files() {
        // Regression for BUG-007: a tree with more than 100 files previously
        // hid most top-level directories because the walk broke at the file cap.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let subdirs = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];
        for dir in &subdirs {
            let path = root.join(dir);
            std::fs::create_dir_all(&path).unwrap();
            // Spread >100 files across the subdirectories so the old recursive
            // cap would trigger before reaching later directories.
            for i in 0..30 {
                std::fs::write(path.join(format!("file_{}.txt", i)), "x").unwrap();
            }
        }

        let result = run_ls(root).await;

        for dir in &subdirs {
            assert!(
                result.output.contains(&format!("  {}/\n", dir)),
                "top-level directory '{}' should appear in output:\n{}",
                dir,
                result.output
            );
        }
        assert_eq!(
            result.metadata.get("dirs").and_then(|v| v.as_u64()),
            Some(6)
        );
        assert_eq!(
            result.metadata.get("truncated").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn includes_empty_directories_and_immediate_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("empty_dir")).unwrap();
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::write(root.join("top.txt"), "x").unwrap();
        std::fs::write(root.join("nested/child.txt"), "x").unwrap();

        let result = run_ls(root).await;

        assert!(result.output.contains("  empty_dir/\n"));
        assert!(result.output.contains("  nested/\n"));
        assert!(result.output.contains("  top.txt\n"));
        // One level only: a nested child must not be listed.
        assert!(!result.output.contains("child.txt"));
    }

    #[tokio::test]
    async fn caps_files_only_and_never_hides_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("adir")).unwrap();
        for i in 0..(LIMIT + 5) {
            std::fs::write(root.join(format!("f_{:04}.txt", i)), "x").unwrap();
        }

        let result = run_ls(root).await;

        assert!(
            result.output.contains("  adir/\n"),
            "directory must still appear"
        );
        assert_eq!(
            result.metadata.get("files").and_then(|v| v.as_u64()),
            Some(LIMIT as u64)
        );
        assert_eq!(
            result.metadata.get("total_files").and_then(|v| v.as_u64()),
            Some((LIMIT + 5) as u64)
        );
        assert_eq!(
            result.metadata.get("truncated").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(result.truncated);
    }

    #[tokio::test]
    async fn ignores_configured_children() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("keep")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();

        let tool = LsTool::new();
        let result = tool
            .execute(
                serde_json::json!({
                    "path": root.to_string_lossy(),
                    "ignore": ["node_modules"],
                }),
                ctx_for(root),
            )
            .await
            .unwrap();

        assert!(result.output.contains("  keep/\n"));
        assert!(!result.output.contains("node_modules"));
    }
}
