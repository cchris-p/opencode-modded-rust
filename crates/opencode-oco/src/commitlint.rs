use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::engine::{ChatMessage, Engine};
use crate::prompt::{generate_commit_string, IDENTITY, INIT_DIFF_PROMPT};
use crate::Runtime;

pub const COMMITLINT_CONFIG_FILE: &str = ".opencommit-commitlint";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitlintLlmConfig {
    pub hash: String,
    pub prompts: Vec<String>,
    pub consistency: BTreeMap<String, Value>,
}

pub async fn commitlint_command(rt: &Runtime, args: &[String]) -> Result<()> {
    let Some(mode) = args.first().map(String::as_str) else {
        print_commitlint_help();
        return Ok(());
    };
    match mode {
        "-h" | "--help" => {
            print_commitlint_help();
            Ok(())
        }
        "get" => {
            let config = read_llm_config(rt)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
            Ok(())
        }
        "force" => configure(rt, true).await,
        other => bail!("Unsupported mode: {other}. Valid modes are: \"force\" and \"get\""),
    }
}

fn print_commitlint_help() {
    println!(
        "Configure the @commitlint prompt module\n\
         \n\
         Usage: oco commitlint <get|force>\n\
         \n\
         get    Print the generated .opencommit-commitlint config\n\
         force  Regenerate .opencommit-commitlint from the local commitlint config"
    );
}

pub fn llm_config_path(rt: &Runtime) -> PathBuf {
    rt.cwd.join(COMMITLINT_CONFIG_FILE)
}

pub fn read_llm_config(rt: &Runtime) -> Result<CommitlintLlmConfig> {
    let path = llm_config_path(rt);
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "{} does not exist. Run `oco commitlint force` first.",
            path.display()
        )
    })?;
    serde_json::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

pub async fn configure(rt: &Runtime, force: bool) -> Result<()> {
    let config = rt.load_config()?;
    let (commitlint_value, source_path) = find_commitlint_config(&rt.cwd)?;

    let serialized = serde_json::to_string(&commitlint_value)?;
    let hash = sha256_hex(&serialized);

    if !force {
        if let Ok(existing) = read_llm_config(rt) {
            if existing.hash == hash {
                println!(
                    "Hashes are the same, no need to update the config. Run \"force\" command to bypass."
                );
                return Ok(());
            }
        }
    }

    let prompts = infer_prompts(&commitlint_value);
    let consistency = generate_consistency(&config, &prompts).await?;

    let mut language_map = BTreeMap::new();
    language_map.insert(config.language_name(), consistency);
    let llm_config = CommitlintLlmConfig {
        hash,
        prompts,
        consistency: language_map,
    };

    let path = llm_config_path(rt);
    fs::write(&path, serde_json::to_string_pretty(&llm_config)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    println!(
        "✔ Read @commitlint configuration from {} and wrote {}",
        source_path.display(),
        path.display()
    );
    Ok(())
}

pub async fn build_prompt_messages(
    rt: &Runtime,
    config: &Config,
    _full_gitmoji: bool,
    _context: &str,
) -> Result<Vec<ChatMessage>> {
    if !llm_config_path(rt).exists() {
        configure(rt, false).await?;
    }
    let llm_config = read_llm_config(rt)?;

    let language = config.language_name();
    let consistency = llm_config
        .consistency
        .get(&language)
        .or_else(|| llm_config.consistency.values().next())
        .cloned()
        .unwrap_or_else(|| json!({}));

    let emoji_instruction = if config.bool_or("OCO_EMOJI", false) {
        if config.bool_or("OCO_EMOJI_POSITION_BEFORE_DESCRIPTION", false) {
            "Use the GitMoji convention and place the emoji at the beginning of the description rather than in the header."
        } else {
            "Use GitMoji convention to preface the commit."
        }
    } else {
        "Do not preface the commit with anything."
    };
    let description_instruction = if config.bool_or("OCO_DESCRIPTION", false) {
        "Add a short description of WHY the changes are done after the commit message. Don't start it with \"This commit\", just describe the changes."
    } else {
        "Don't add any descriptions to the commit, only commit message."
    };
    let why = if config.bool_or("OCO_WHY", false) {
        "and WHY the changes were done"
    } else {
        ""
    };
    let one_line = if config.bool_or("OCO_ONE_LINE_COMMIT", false) {
        "Craft a concise commit message that encapsulates all changes made, with an emphasis on the primary updates. If the modifications share a common theme or scope, mention it succinctly; otherwise, leave the scope out to maintain focus. The goal is to provide a clear and unified overview of the changes in a one single message, without diverging into a list of commit per file change."
    } else {
        ""
    };
    let omit_scope = if config.bool_or("OCO_OMIT_SCOPE", false) {
        "Do not include a scope in the commit message format. Use the format: <type>: <subject>"
    } else {
        ""
    };
    let structure = if config.bool_or("OCO_OMIT_SCOPE", false) {
        "\n- Header of commit is composed of type and subject: <type-of-commit>: <subject-of-commit>\n- Description of commit is composed of body and footer (optional): <body-of-commit>\\n<footer(s)-of-commit>"
    } else {
        "\n- Header of commit is composed of type, scope, subject: <type-of-commit>(<scope-of-commit>): <subject-of-commit>\n- Description of commit is composed of body and footer (optional): <body-of-commit>\\n<footer(s)-of-commit>"
    };

    let system = format!(
        "{IDENTITY} Your mission is to create clean and comprehensive commit messages in the given @commitlint convention and explain WHAT were the changes {why}. I'll send you an output of 'git diff --staged' command, and you convert it into a commit message.\n{emoji_instruction}\n{description_instruction}\nUse the present tense. Use {language} to answer.\n{one_line}\n{omit_scope}\nYou will strictly follow the following conventions to generate the content of the commit message:\n- {}\n\nThe conventions refers to the following structure of commit message:\n{structure}",
        llm_config.prompts.join("\n- "),
    );

    Ok(vec![
        ChatMessage::system(system),
        ChatMessage::user(INIT_DIFF_PROMPT),
        consistency_prompt(config, &consistency),
    ])
}

fn consistency_prompt(config: &Config, consistency: &Value) -> ChatMessage {
    let omit_scope = config.bool_or("OCO_OMIT_SCOPE", false);
    let one_line = config.bool_or("OCO_ONE_LINE_COMMIT", false);

    let fix = string_field(consistency, "commitFix");
    let feat = string_field(consistency, "commitFeat");
    let fix_omit = string_field(consistency, "commitFixOmitScope");
    let feat_omit = string_field(consistency, "commitFeatOmitScope");
    let description = string_field(consistency, "commitDescription");

    let fix_message = if omit_scope && !fix_omit.is_empty() {
        fix_omit
    } else {
        fix
    };
    let feat_message = if omit_scope && !feat_omit.is_empty() {
        feat_omit
    } else {
        feat
    };

    let fix_line = if fix_message.is_empty() {
        String::new()
    } else {
        generate_commit_string(config, "fix", &fix_message)
    };
    let feat_line = if one_line || feat_message.is_empty() {
        String::new()
    } else {
        generate_commit_string(config, "feat", &feat_message)
    };
    let description_line = if config.bool_or("OCO_DESCRIPTION", false) {
        description
    } else {
        String::new()
    };

    let content = [fix_line, feat_line, description_line]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    ChatMessage::assistant(content)
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

async fn generate_consistency(config: &Config, prompts: &[String]) -> Result<Value> {
    let language = config.language_name();
    let omit_scope = config.bool_or("OCO_OMIT_SCOPE", false);
    let header = if omit_scope {
        "`<type>: <subject>`"
    } else {
        "`<type>(<scope>): <subject>`"
    };

    let system = format!(
        "{IDENTITY} Your mission is to create clean and comprehensive commit messages for two different changes in a single codebase and output them in the provided JSON format: one for a bug fix and another for a new feature.\n\nHere are the specific requirements and conventions that should be strictly followed:\n\nCommit Message Conventions:\n- The commit message consists of three parts: Header, Body, and Footer.\n- Header:\n  - Format: {header}\n- {}\n\nJSON Output Format:\n- The JSON output should contain the commit messages for a bug fix and a new feature in the following format:\n```json\n{{\n  \"localLanguage\": \"{language}\",\n  \"commitFix\": \"<Header of commit for bug fix with scope>\",\n  \"commitFeat\": \"<Header of commit for feature with scope>\",\n  \"commitFixOmitScope\": \"<Header of commit for bug fix without scope>\",\n  \"commitFeatOmitScope\": \"<Header of commit for feature without scope>\",\n  \"commitDescription\": \"<Description of commit for both the bug fix and the feature>\"\n}}\n```\n- The \"commitDescription\" should not include the commit message's header, only the description.\n- Description should not be more than 74 characters.\n\nAdditional Details:\n- Changing the variable 'port' to uppercase 'PORT' is considered a bug fix.\n- Allowing the server to listen on a port specified through the environment variable is considered a new feature.\n\nExample Git Diff is to follow:",
        prompts.join("\n- "),
    );

    let messages = vec![
        ChatMessage::system(system),
        ChatMessage::user(INIT_DIFF_PROMPT),
    ];
    let engine = Engine::from_config(config)?;
    let raw = engine.generate(&messages).await?;
    let json_block = extract_json_block(&raw);
    serde_json::from_str(&json_block).with_context(|| {
        let preview: String = raw.chars().take(400).collect();
        format!("failed to parse the @commitlint consistency JSON returned by the model: {preview}")
    })
}

fn extract_json_block(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(start) = trimmed.find("```json") {
        let rest = &trimmed[start + 7..];
        if let Some(end) = rest.find("```") {
            return rest[..end].trim().to_string();
        }
    }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end >= start {
                return trimmed[start..=end].to_string();
            }
        }
    }
    trimmed.to_string()
}

fn find_commitlint_config(cwd: &Path) -> Result<(Value, PathBuf)> {
    let json_candidates = [
        ".commitlintrc",
        ".commitlintrc.json",
        ".commitlintrc.jsonc",
        "commitlint.config.json",
    ];
    for candidate in json_candidates {
        let path = cwd.join(candidate);
        if path.exists() {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let value: Value = serde_json::from_str(&text).with_context(|| {
                format!(
                    "{} is not valid JSON. Native oco supports JSON commitlint configs; export JavaScript/YAML configs to .commitlintrc.json.",
                    path.display()
                )
            })?;
            return Ok((value, path));
        }
    }

    for candidate in [
        ".commitlintrc.yaml",
        ".commitlintrc.yml",
        "commitlint.config.js",
        "commitlint.config.cjs",
        "commitlint.config.mjs",
        ".commitlintrc.js",
        ".commitlintrc.cjs",
        ".commitlintrc.mjs",
    ] {
        let path = cwd.join(candidate);
        if path.exists() {
            bail!(
                "{} uses a JavaScript/YAML commitlint config, which native oco does not parse. Export the resolved config to .commitlintrc.json (for example with @commitlint/load) and rerun `oco commitlint force`.",
                path.display()
            );
        }
    }

    let package_path = cwd.join("package.json");
    if package_path.exists() {
        let text = fs::read_to_string(&package_path)
            .with_context(|| format!("failed to read {}", package_path.display()))?;
        let package: Value = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", package_path.display()))?;
        if let Some(value) = package.get("commitlint") {
            return Ok((value.clone(), package_path));
        }
    }

    bail!(
        "Failed to load @commitlint config. Please check the following:\n * A valid @commitlint JSON configuration exists (.commitlintrc, .commitlintrc.json, or package.json#commitlint).\n * JavaScript/YAML configs must be exported to JSON first."
    )
}

pub fn infer_prompts(config: &Value) -> Vec<String> {
    let Some(rules) = config.get("rules").and_then(Value::as_object) else {
        return Vec::new();
    };
    let prompt_config = config.get("prompt");
    rules
        .iter()
        .filter_map(|(rule_name, rule)| infer_rule_prompt(rule_name, rule, prompt_config))
        .collect()
}

fn infer_rule_prompt(
    rule_name: &str,
    rule: &Value,
    prompt_config: Option<&Value>,
) -> Option<String> {
    let tuple = rule.as_array()?;
    let severity = tuple.first().and_then(Value::as_u64).unwrap_or(2);
    if severity == 0 {
        return None;
    }
    let applicable = tuple
        .get(1)
        .and_then(Value::as_str)
        .unwrap_or("always")
        .to_string();
    let value = tuple.get(2).cloned().unwrap_or(Value::Null);

    Some(render_rule(rule_name, &applicable, &value, prompt_config))
}

fn render_rule(
    name: &str,
    applicable: &str,
    value: &Value,
    prompt_config: Option<&Value>,
) -> String {
    match name {
        "body-case" => case_rule("body", applicable, value),
        "body-empty" => empty_rule("body", applicable),
        "body-full-stop" => full_stop_rule("body", applicable, value),
        "body-leading-blank" => blank_line_rule("body", applicable),
        "body-max-length" => max_length_rule("body", applicable, value),
        "body-max-line-length" => format!("Each line of the body should {applicable} have {} characters or less.", value_text(value)),
        "body-min-length" => min_length_rule("body", applicable, value),
        "footer-case" => case_rule("footer", applicable, value),
        "footer-empty" => empty_rule("footer", applicable),
        "footer-leading-blank" => blank_line_rule("footer", applicable),
        "footer-max-length" => max_length_rule("footer", applicable, value),
        "footer-max-line-length" => format!("Each line of the footer should {applicable} have {} characters or less.", value_text(value)),
        "footer-min-length" => min_length_rule("footer", applicable, value),
        "header-case" => case_rule("header", applicable, value),
        "header-full-stop" => full_stop_rule("header", applicable, value),
        "header-max-length" => max_length_rule("header", applicable, value),
        "header-min-length" => min_length_rule("header", applicable, value),
        "references-empty" => empty_rule("references section", applicable),
        "scope-case" => case_rule("scope", applicable, value),
        "scope-empty" => empty_rule("scope", applicable),
        "scope-enum" => enum_rule("type", applicable, value),
        "scope-max-length" => max_length_rule("scope", applicable, value),
        "scope-min-length" => min_length_rule("scope", applicable, value),
        "signed-off-by" => format!("The commit message should {applicable} have a \"Signed-off-by\" line with the value \"{}\".", value_text(value)),
        "subject-case" => case_rule("subject", applicable, value),
        "subject-empty" => empty_rule("subject", applicable),
        "subject-full-stop" => full_stop_rule("subject", applicable, value),
        "subject-max-length" => max_length_rule("subject", applicable, value),
        "subject-min-length" => min_length_rule("subject", applicable, value),
        "type-case" => case_rule("type", applicable, value),
        "type-empty" => empty_rule("type", applicable),
        "type-enum" => enum_type_rule("type", applicable, value, prompt_config),
        "type-max-length" => max_length_rule("type", applicable, value),
        "type-min-length" => min_length_rule("type", applicable, value),
        other => format!("Please manually set the prompt for rule \"{other}\"."),
    }
}

fn case_rule(name: &str, applicable: &str, value: &Value) -> String {
    let case = match value {
        Value::Array(items) => format!(
            "one of the following case:\n  - {}",
            items
                .iter()
                .map(value_text)
                .collect::<Vec<_>>()
                .join("\n  - ")
        ),
        other => format!("{} case", value_text(other)),
    };
    format!("The {name} should {applicable} be in {case}.")
}

fn empty_rule(name: &str, applicable: &str) -> String {
    format!("The {name} should {applicable} be empty.")
}

fn enum_rule(name: &str, applicable: &str, value: &Value) -> String {
    format!(
        "The {name} should {applicable} be one of the following values:\n  - {}.",
        value_list(value)
    )
}

fn enum_type_rule(
    name: &str,
    applicable: &str,
    value: &Value,
    prompt_config: Option<&Value>,
) -> String {
    let rendered = match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                let item_text = value_text(item);
                if let Some(description) = type_description(prompt_config, &item_text) {
                    format!("{item_text} ({description})")
                } else {
                    item_text
                }
            })
            .collect::<Vec<_>>()
            .join("\n  - "),
        other => value_text(other),
    };
    format!("The {name} should {applicable} be one of the following values:\n  - {rendered}.")
}

fn type_description(prompt_config: Option<&Value>, type_name: &str) -> Option<String> {
    prompt_config?
        .get("questions")?
        .get("type")?
        .get("enum")?
        .get(type_name)?
        .get("description")?
        .as_str()
        .map(ToString::to_string)
}

fn full_stop_rule(name: &str, applicable: &str, value: &Value) -> String {
    format!(
        "The {name} should {applicable} end with '{}'.",
        value_text(value)
    )
}

fn max_length_rule(name: &str, applicable: &str, value: &Value) -> String {
    format!(
        "The {name} should {applicable} have {} characters or less.",
        value_text(value)
    )
}

fn min_length_rule(name: &str, applicable: &str, value: &Value) -> String {
    format!(
        "The {name} should {applicable} have {} characters or more.",
        value_text(value)
    )
}

fn blank_line_rule(name: &str, applicable: &str) -> String {
    format!("There should {applicable} be a blank line at the beginning of the {name}.")
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn value_list(value: &Value) -> String {
    match value {
        Value::Array(items) => items
            .iter()
            .map(value_text)
            .collect::<Vec<_>>()
            .join("\n  - "),
        other => value_text(other),
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Value {
        json!({
            "rules": {
                "type-enum": [2, "always", ["feat", "fix", "docs"]],
                "subject-empty": [2, "never"],
                "header-max-length": [2, "always", 72],
                "body-leading-blank": [0, "always"]
            },
            "prompt": {
                "questions": {
                    "type": {
                        "enum": {
                            "feat": { "description": "A new feature" }
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn infers_prompts_for_known_rules() {
        let prompts = infer_prompts(&sample_config());
        assert_eq!(prompts.len(), 3);
        assert!(prompts.iter().any(|p| p.contains("feat (A new feature)")));
        assert!(prompts
            .iter()
            .any(|p| p.contains("subject should never be empty")));
        assert!(prompts.iter().any(|p| p.contains("72 characters or less")));
    }

    #[test]
    fn extracts_fenced_json() {
        let raw = "Here you go:\n```json\n{\"commitFix\": \"x\"}\n```\ntrailing";
        assert_eq!(extract_json_block(raw), "{\"commitFix\": \"x\"}");
    }
}
