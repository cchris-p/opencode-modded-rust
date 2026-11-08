use anyhow::{bail, Result};
use std::io::{self, Write};

use crate::config::Config;
use crate::engine::{ChatMessage, Engine};
use crate::git;
use crate::prompt;
use crate::text;
use crate::Runtime;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommitArgs {
    pub yes: bool,
    pub fgm: bool,
    pub context: String,
    pub extra: Vec<String>,
}

pub fn parse_commit_args(args: &[String]) -> Result<CommitArgs> {
    let mut parsed = CommitArgs::default();
    let mut index = 0;

    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--" => {
                parsed.extra.extend(args[index..].iter().cloned());
                break;
            }
            "-y" | "--yes" => parsed.yes = true,
            "--fgm" => parsed.fgm = true,
            "-c" | "--context" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    bail!("{arg} requires a value");
                };
                parsed.context = value.clone();
            }
            _ if arg.starts_with("--context=") => {
                parsed.context = arg["--context=".len()..].to_string();
            }
            _ if arg.starts_with("-c=") => {
                parsed.context = arg["-c=".len()..].to_string();
            }
            _ if arg.starts_with("--yes=") => {
                parsed.yes = parse_bool_flag(&arg["--yes=".len()..])?;
            }
            _ if arg.starts_with("-y=") => {
                parsed.yes = parse_bool_flag(&arg["-y=".len()..])?;
            }
            _ if arg.starts_with("--fgm=") => {
                parsed.fgm = parse_bool_flag(&arg["--fgm=".len()..])?;
            }
            _ => parsed.extra.push(args[index].clone()),
        }
        index += 1;
    }

    Ok(parsed)
}

fn parse_bool_flag(value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        other => bail!("invalid boolean value: {other}"),
    }
}

pub fn apply_message_template(
    config: &Config,
    message: &str,
    extra_args: &mut Vec<String>,
) -> String {
    let placeholder = config.string_or("OCO_MESSAGE_TEMPLATE_PLACEHOLDER", "$msg");
    if placeholder.is_empty() {
        return message.to_string();
    }
    if let Some(index) = extra_args
        .iter()
        .position(|argument| argument.contains(&placeholder))
    {
        let template = extra_args.remove(index);
        return template.replace(&placeholder, message);
    }
    message.to_string()
}

/// Build the provider request and generate one commit message.
pub async fn generate_message(
    rt: &Runtime,
    config: &Config,
    diff: &str,
    full_gitmoji: bool,
    context: &str,
) -> Result<String> {
    let prompt_module = config.string_or("OCO_PROMPT_MODULE", "conventional-commit");
    let mut messages = if prompt_module == "@commitlint" {
        crate::commitlint::build_prompt_messages(rt, config, full_gitmoji, context).await?
    } else {
        prompt::build_commit_messages(config, full_gitmoji, context)
    };

    let prompt_tokens: u64 = messages
        .iter()
        .map(|message| text::estimate_tokens(&message.content) + 4)
        .sum();
    let max_input = config.u64_or("OCO_TOKENS_MAX_INPUT", 4096);
    let max_output = config.u64_or("OCO_TOKENS_MAX_OUTPUT", 500);
    let max_request_tokens = max_input
        .saturating_sub(20)
        .saturating_sub(prompt_tokens)
        .saturating_sub(max_output);
    if max_request_tokens == 0 {
        bail!(
            "OCO_TOKENS_MAX_INPUT ({max_input}) is too small for the prompt and OCO_TOKENS_MAX_OUTPUT ({max_output})"
        );
    }

    let (diff, truncated) = text::truncate_to_tokens(diff, max_request_tokens);
    if truncated {
        tracing::debug!(
            max_request_tokens,
            "staged diff truncated once to fit the token budget"
        );
    }
    messages.push(ChatMessage::user(diff));

    let engine = Engine::from_config(config)?;
    let raw = engine.generate(&messages).await?;
    let cleaned = text::clean_message(&raw);
    if cleaned.is_empty() {
        bail!("the provider returned an empty commit message");
    }
    Ok(cleaned)
}

pub async fn commit_command(rt: &Runtime, args: &[String]) -> Result<()> {
    let parsed = parse_commit_args(args)?;
    let config = rt.load_config()?;

    git::assert_git_repo(&rt.cwd)?;

    let mut staged = git::get_staged_files(&rt.cwd)?;
    let changed = git::get_changed_files(&rt.cwd)?;

    if staged.is_empty() && changed.is_empty() {
        bail!("No changes detected");
    }

    if staged.is_empty() {
        if rt.interactive {
            if !confirm(
                "No files are staged. Do you want to stage all files and generate a commit message?",
                true,
            )? {
                bail!("no files staged");
            }
        } else {
            eprintln!("No files are staged; staging all changed files.");
        }
        git::git_add(&rt.cwd, &changed)?;
        staged = git::get_staged_files(&rt.cwd)?;
    }

    if staged.is_empty() {
        bail!("No changes detected");
    }

    let diff = git::get_diff(&rt.cwd, &staged)?;
    if diff.trim().is_empty() {
        bail!("No staged diff to generate a commit message from");
    }

    let mut message = generate_message(rt, &config, &diff, parsed.fgm, &parsed.context).await?;
    let mut extra = parsed.extra.clone();
    message = apply_message_template(&config, &message, &mut extra);

    println!("Generated commit message:\n——————————————————\n{message}\n——————————————————");

    let final_message = if parsed.yes {
        message
    } else {
        if !rt.interactive {
            bail!("commit confirmation requires a terminal; rerun with --yes");
        }
        confirm_message(&message)?
    };

    let output = git::commit(&rt.cwd, &final_message, &extra)?;
    if !output.is_empty() {
        println!("{output}");
    }

    maybe_push(rt, &config)?;
    Ok(())
}

fn maybe_push(rt: &Runtime, config: &Config) -> Result<()> {
    if !config.bool_or("OCO_GITPUSH", true) || !rt.interactive {
        return Ok(());
    }

    let remotes = git::get_remotes(&rt.cwd)?;
    if remotes.is_empty() {
        return Ok(());
    }

    let remote = if remotes.len() == 1 {
        if !confirm("Do you want to run `git push`?", false)? {
            println!("`git push` aborted");
            return Ok(());
        }
        remotes[0].clone()
    } else {
        println!("Available remotes: {}", remotes.join(", "));
        println!("Choose a remote to push to (empty to skip):");
        let answer = read_line()?;
        let answer = answer.trim();
        if answer.is_empty() {
            return Ok(());
        }
        if !remotes.iter().any(|remote| remote == answer) {
            bail!("unknown remote: {answer}");
        }
        answer.to_string()
    };

    let set_upstream = !git::has_upstream_branch(&rt.cwd)?;
    git::push(&rt.cwd, Some(&remote), set_upstream)?;
    println!("✔ successfully pushed all commits to {remote}");
    Ok(())
}

fn confirm_message(message: &str) -> Result<String> {
    loop {
        print!("Commit with this message? [Y/n/e] ");
        io::stdout().flush().ok();
        let answer = read_line()?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => return Ok(message.to_string()),
            "n" | "no" => bail!("commit aborted"),
            "e" | "edit" => {
                print!("Please edit the commit message: ");
                io::stdout().flush().ok();
                let edited = read_line()?;
                if edited.trim().is_empty() {
                    continue;
                }
                return Ok(edited.trim_end_matches(['\r', '\n']).to_string());
            }
            _ => continue,
        }
    }
}

fn confirm(question: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{question} {suffix} ");
    io::stdout().flush().ok();
    let answer = read_line()?;
    let answer = answer.trim().to_ascii_lowercase();
    if answer.is_empty() {
        return Ok(default_yes);
    }
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

fn read_line() -> Result<String> {
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn strips_known_flags_and_keeps_git_args() {
        let args = vec![
            "-c".to_string(),
            "why".to_string(),
            "--yes".to_string(),
            "--fgm".to_string(),
            "--no-verify".to_string(),
            "#205: $msg".to_string(),
        ];
        let parsed = parse_commit_args(&args).unwrap();
        assert!(parsed.yes);
        assert!(parsed.fgm);
        assert_eq!(parsed.context, "why");
        assert_eq!(parsed.extra, vec!["--no-verify", "#205: $msg"]);
    }

    #[test]
    fn supports_equals_forms() {
        let args = vec!["--context=abc".to_string(), "-y=true".to_string()];
        let parsed = parse_commit_args(&args).unwrap();
        assert_eq!(parsed.context, "abc");
        assert!(parsed.yes);
    }

    #[test]
    fn applies_and_removes_template_argument() {
        let mut config = Config::defaults();
        config.set(
            "OCO_MESSAGE_TEMPLATE_PLACEHOLDER",
            Value::String("$msg".into()),
        );
        let mut extra = vec!["--no-verify".to_string(), "#205: $msg".to_string()];
        let message = apply_message_template(&config, "feat: add", &mut extra);
        assert_eq!(message, "#205: feat: add");
        assert_eq!(extra, vec!["--no-verify"]);
    }

    #[test]
    fn forwards_arguments_after_double_dash() {
        let args = vec!["--".to_string(), "--yes".to_string()];
        let parsed = parse_commit_args(&args).unwrap();
        assert!(!parsed.yes);
        assert_eq!(parsed.extra, vec!["--", "--yes"]);
    }
}
