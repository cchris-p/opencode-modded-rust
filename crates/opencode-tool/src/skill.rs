use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::{PermissionRequest, Tool, ToolContext, ToolError, ToolResult};
use opencode_config::load_config;

pub struct SkillTool;

#[derive(Debug, Serialize, Deserialize)]
struct SkillInput {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "skill_name")]
    #[serde(default)]
    skill_name: Option<String>,
    #[serde(default)]
    arguments: Option<serde_json::Value>,
    #[serde(default)]
    prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableSkill {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
struct SkillInfo {
    name: String,
    description: String,
    content: String,
    location: PathBuf,
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from).or_else(|| {
        #[cfg(windows)]
        {
            std::env::var_os("USERPROFILE").map(PathBuf::from)
        }
        #[cfg(not(windows))]
        {
            None
        }
    })
}

fn normalize_existing_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn detect_worktree_root(start: &Path) -> PathBuf {
    let mut current = normalize_existing_path(start);
    loop {
        if current.join(".git").exists() {
            return current;
        }
        let Some(parent) = current.parent() else {
            return current;
        };
        if parent == current {
            return current;
        }
        current = parent.to_path_buf();
    }
}

fn walk_up_directories(start: &Path, stop: &Path) -> Vec<PathBuf> {
    let mut current = normalize_existing_path(start);
    let stop = normalize_existing_path(stop);
    let mut result = Vec::new();

    loop {
        result.push(current.clone());
        if current == stop {
            break;
        }
        let Some(parent) = current.parent() else {
            break;
        };
        if parent == current {
            break;
        }
        current = parent.to_path_buf();
    }

    result
}

fn resolve_skill_path(base: &Path, raw: &str) -> PathBuf {
    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(stripped);
        }
    }

    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn collect_project_skill_roots(base: &Path) -> Vec<PathBuf> {
    let worktree = detect_worktree_root(base);
    let mut roots = Vec::new();

    for dir in walk_up_directories(base, &worktree).into_iter().rev() {
        roots.push(dir.join(".agents/skills"));
        roots.push(dir.join(".claude/skills"));
        roots.push(dir.join(".opencode/skill"));
        roots.push(dir.join(".opencode/skills"));
    }

    roots
}

fn collect_skill_roots(base: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = home_dir() {
        roots.push(home.join(".agents/skills"));
        roots.push(home.join(".claude/skills"));
    }

    // Global config directory (e.g. ~/.config/opencode/skills)
    if let Some(config_dir) = dirs::config_dir() {
        roots.push(config_dir.join("opencode/skill"));
        roots.push(config_dir.join("opencode/skills"));
    }

    // Home .opencode directory
    if let Some(home) = home_dir() {
        roots.push(home.join(".opencode/skill"));
        roots.push(home.join(".opencode/skills"));
    }

    roots.extend(collect_project_skill_roots(base));

    if let Ok(config) = load_config(base) {
        if let Some(skills) = config.skills {
            for raw in skills.paths {
                roots.push(resolve_skill_path(base, &raw));
            }
        }
    }

    let mut deduped = Vec::new();
    for root in roots {
        if !deduped.contains(&root) {
            deduped.push(root);
        }
    }
    deduped
}

fn resolve_skill_name(input: SkillInput) -> Result<ResolvedSkillInput, ToolError> {
    let skill_name = input
        .skill_name
        .or(input.name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ToolError::InvalidArguments(
                "Missing required field: provide either 'skill_name' or 'name'".to_string(),
            )
        })?;

    Ok(ResolvedSkillInput {
        skill_name,
        arguments: input.arguments,
        prompt: input.prompt,
    })
}

struct ResolvedSkillInput {
    skill_name: String,
    arguments: Option<serde_json::Value>,
    prompt: Option<String>,
}

fn parse_frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix(&format!("{key}:")) {
            let value = value.trim();
            if value.len() >= 2 {
                if (value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    return Some(value[1..value.len() - 1].to_string());
                }
            }
            return Some(value.to_string());
        }
    }
    None
}

fn parse_skill_file(path: &Path) -> Option<SkillInfo> {
    let raw = fs::read_to_string(path).ok()?;
    let normalized = raw.replace("\r\n", "\n");
    let mut lines = normalized.lines();

    if lines.next()?.trim() != "---" {
        return None;
    }

    let mut frontmatter_lines = Vec::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        frontmatter_lines.push(line);
    }
    if !closed {
        return None;
    }

    let frontmatter = frontmatter_lines.join("\n");
    let content = lines.collect::<Vec<_>>().join("\n");
    let name = parse_frontmatter_value(&frontmatter, "name")?;
    let description = parse_frontmatter_value(&frontmatter, "description")?;

    Some(SkillInfo {
        name,
        description,
        content: content.trim().to_string(),
        location: path.to_path_buf(),
    })
}

fn scan_skill_root(root: &Path) -> Vec<SkillInfo> {
    if !root.exists() || !root.is_dir() {
        return Vec::new();
    }

    let mut skill_files: Vec<PathBuf> = WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == "SKILL.md")
                .unwrap_or(false)
        })
        .collect();
    skill_files.sort();

    skill_files
        .into_iter()
        .filter_map(|path| parse_skill_file(&path))
        .collect()
}

fn discover_skills(base: &Path) -> Vec<SkillInfo> {
    let mut by_name: HashMap<String, SkillInfo> = HashMap::new();
    for root in collect_skill_roots(base) {
        for skill in scan_skill_root(&root) {
            by_name.insert(skill.name.clone(), skill);
        }
    }

    let mut skills: Vec<SkillInfo> = by_name.into_values().collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn sample_skill_files(skill: &SkillInfo, limit: usize) -> Vec<PathBuf> {
    let Some(base_dir) = skill.location.parent() else {
        return Vec::new();
    };

    let mut files: Vec<PathBuf> = WalkDir::new(base_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name != "SKILL.md")
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    files.truncate(limit);
    files
}

#[async_trait]
impl Tool for SkillTool {
    fn id(&self) -> &str {
        "skill"
    }

    fn description(&self) -> &str {
        "Load and execute a skill (predefined expertise module). Skills provide specialized knowledge for specific tasks."
    }

    fn parameters(&self) -> serde_json::Value {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let skills = discover_skills(&base);
        let skill_names: Vec<String> = skills.into_iter().map(|s| s.name).collect();

        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the skill to load",
                    "enum": skill_names
                },
                "skill_name": {
                    "type": "string",
                    "description": "Name of the skill to load",
                    "enum": skill_names
                },
                "arguments": {
                    "type": "object",
                    "description": "Arguments to pass to the skill"
                },
                "prompt": {
                    "type": "string",
                    "description": "Additional prompt/instructions for the skill"
                }
            },
            "anyOf": [
                {"required": ["skill_name"]},
                {"required": ["name"]}
            ]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let raw_input: SkillInput =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let input = resolve_skill_name(raw_input)?;

        let skills = discover_skills(Path::new(&ctx.directory));

        let skill = skills
            .iter()
            .find(|s| s.name == input.skill_name)
            .ok_or_else(|| {
                ToolError::InvalidArguments(format!(
                    "Unknown skill: {}. Available skills: {}",
                    input.skill_name,
                    skills
                        .iter()
                        .map(|s| &s.name)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;

        ctx.ask_permission(
            PermissionRequest::new("skill")
                .with_pattern(&skill.name)
                .with_always(&skill.name)
                .with_metadata("description", serde_json::json!(&skill.description)),
        )
        .await?;

        let mut output = format!("<skill_content name=\"{}\">\n\n", skill.name);
        output.push_str(&format!("# Skill: {}\n\n", skill.name));
        output.push_str(&skill.content);
        output.push_str("\n\n");
        output.push_str(&format!(
            "Base directory for this skill: {}\n",
            skill
                .location
                .parent()
                .unwrap_or(Path::new(&ctx.directory))
                .display()
        ));
        output.push_str(
            "Relative paths in this skill (e.g., scripts/, references/) are relative to this base directory.\n",
        );
        output.push_str("Note: file list is sampled.\n\n");

        let sampled_files = sample_skill_files(skill, 10);
        output.push_str("<skill_files>\n");
        for file in sampled_files {
            output.push_str(&format!("<file>{}</file>\n", file.display()));
        }
        output.push_str("</skill_files>\n");

        if let Some(ref args) = input.arguments {
            output.push_str(&format!(
                "**Arguments:**\n```json\n{}\n```\n\n",
                serde_json::to_string_pretty(args).unwrap_or_default()
            ));
        }

        if let Some(ref prompt) = input.prompt {
            output.push_str(&format!("**Additional Instructions:**\n{}\n\n", prompt));
        }

        output.push_str("\n</skill_content>");

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("name".to_string(), serde_json::json!(&skill.name));
        metadata.insert(
            "dir".to_string(),
            serde_json::json!(skill
                .location
                .parent()
                .unwrap_or(Path::new(&ctx.directory))
                .to_string_lossy()
                .to_string()),
        );
        metadata.insert(
            "location".to_string(),
            serde_json::json!(skill.location.to_string_lossy().to_string()),
        );

        Ok(ToolResult {
            title: format!("Loaded skill: {}", skill.name),
            output,
            metadata,
            truncated: false,
        })
    }
}

impl Default for SkillTool {
    fn default() -> Self {
        Self
    }
}

pub fn list_available_skills() -> Vec<AvailableSkill> {
    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    list_available_skills_for_base(&base)
}

pub fn list_available_skills_for_base(base: &Path) -> Vec<AvailableSkill> {
    discover_skills(base)
        .into_iter()
        .map(|s| AvailableSkill {
            name: s.name,
            description: s.description,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_skill_file_reads_frontmatter_and_body() {
        let dir = tempdir().unwrap();
        let skill_path = dir.path().join("SKILL.md");
        fs::write(
            &skill_path,
            r#"---
name: reviewer
description: "Review code changes"
---

# Reviewer

Do a thorough review.
"#,
        )
        .unwrap();

        let parsed = parse_skill_file(&skill_path).unwrap();
        assert_eq!(parsed.name, "reviewer");
        assert_eq!(parsed.description, "Review code changes");
        assert!(parsed.content.contains("Do a thorough review."));
    }

    #[test]
    fn discover_skills_loads_project_and_config_paths() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let project_skill = root.join(".opencode/skills/local/SKILL.md");
        fs::create_dir_all(project_skill.parent().unwrap()).unwrap();
        fs::write(
            &project_skill,
            r#"---
name: local-skill
description: local
---
project content
"#,
        )
        .unwrap();

        let extra_root = root.join("custom-skills");
        let extra_skill = extra_root.join("remote/SKILL.md");
        fs::create_dir_all(extra_skill.parent().unwrap()).unwrap();
        fs::write(
            &extra_skill,
            r#"---
name: custom-skill
description: custom
---
custom content
"#,
        )
        .unwrap();

        fs::write(
            root.join("opencode.json"),
            r#"{
  "skills": {
    "paths": ["custom-skills"]
  }
}"#,
        )
        .unwrap();

        let discovered = discover_skills(root);
        let names: Vec<String> = discovered.into_iter().map(|s| s.name).collect();

        assert!(names.contains(&"local-skill".to_string()));
        assert!(names.contains(&"custom-skill".to_string()));
    }

    #[test]
    fn discover_skills_walks_up_to_worktree_root() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join(".git"), "gitdir").unwrap();

        let root_skill = root.join(".opencode/skills/root/SKILL.md");
        fs::create_dir_all(root_skill.parent().unwrap()).unwrap();
        fs::write(
            &root_skill,
            r#"---
name: shared-skill
description: root copy
---
root content
"#,
        )
        .unwrap();

        let nested = root.join("apps/cli");
        fs::create_dir_all(&nested).unwrap();

        let discovered = discover_skills(&nested);
        let skill = discovered
            .into_iter()
            .find(|skill| skill.name == "shared-skill")
            .unwrap();

        assert_eq!(skill.description, "root copy");
    }

    #[test]
    fn nearer_project_roots_override_ancestor_duplicates() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join(".git"), "gitdir").unwrap();

        let root_skill = root.join(".opencode/skills/reviewer/SKILL.md");
        fs::create_dir_all(root_skill.parent().unwrap()).unwrap();
        fs::write(
            &root_skill,
            r#"---
name: reviewer
description: root reviewer
---
root
"#,
        )
        .unwrap();

        let nested = root.join("apps/cli");
        let nested_skill = nested.join(".opencode/skills/reviewer/SKILL.md");
        fs::create_dir_all(nested_skill.parent().unwrap()).unwrap();
        fs::write(
            &nested_skill,
            r#"---
name: reviewer
description: nested reviewer
---
nested
"#,
        )
        .unwrap();

        let discovered = discover_skills(&nested);
        let skill = discovered
            .into_iter()
            .find(|skill| skill.name == "reviewer")
            .unwrap();

        assert_eq!(skill.description, "nested reviewer");
    }

    #[test]
    fn resolve_skill_name_accepts_reference_name_alias() {
        let resolved = resolve_skill_name(SkillInput {
            name: Some("reviewer".to_string()),
            skill_name: None,
            arguments: None,
            prompt: None,
        })
        .unwrap();

        assert_eq!(resolved.skill_name, "reviewer");
    }
}
