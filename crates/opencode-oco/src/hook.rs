use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::git;
use crate::{commit, Runtime};

const HOOK_NAME: &str = "prepare-commit-msg";
const MANAGED_MARKER: &str = "# opencommit-managed prepare-commit-msg hook";

pub async fn hook_command(rt: &Runtime, args: &[String]) -> Result<()> {
    let Some(mode) = args.first().map(String::as_str) else {
        print_hook_help();
        return Ok(());
    };
    match mode {
        "-h" | "--help" => {
            print_hook_help();
            Ok(())
        }
        "set" => hook_set(rt),
        "unset" => hook_unset(rt),
        "run" => hook_run(rt, &args[1..]).await,
        other => bail!("Unsupported mode: {other}. Supported modes are: 'set', 'unset', or 'run'"),
    }
}

fn hook_path(rt: &Runtime) -> Result<PathBuf> {
    Ok(git::hooks_dir(&rt.cwd)?.join(HOOK_NAME))
}

fn current_invocation() -> Result<String> {
    let exe = std::env::current_exe().context("failed to resolve the current executable")?;
    let name = exe
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    if name == "oco" || name == "opencommit" {
        Ok(format!("\"{}\"", exe.display()))
    } else {
        Ok(format!("\"{}\" oco", exe.display()))
    }
}

fn hook_script() -> Result<String> {
    let invocation = current_invocation()?;
    Ok(format!(
        "#!/bin/sh\n{MANAGED_MARKER}\nexec {invocation} hook run \"$@\"\n"
    ))
}

fn hook_set(rt: &Runtime) -> Result<()> {
    git::assert_git_repo(&rt.cwd)?;
    let path = hook_path(rt)?;
    let script = hook_script()?;

    if path.exists() {
        let existing = fs::read_to_string(&path).unwrap_or_default();
        if existing.contains(MANAGED_MARKER) {
            if existing == script {
                println!(
                    "OpenCommit is already set as '{HOOK_NAME}' at {}",
                    path.display()
                );
            } else {
                fs::write(&path, script)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                make_executable(&path)?;
                println!(
                    "OpenCommit '{HOOK_NAME}' hook updated at {}",
                    path.display()
                );
            }
            return Ok(());
        }
        bail!(
            "A different {HOOK_NAME} hook is already set at {}. It was preserved; remove it before setting OpenCommit as the {HOOK_NAME} hook.",
            path.display()
        );
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, script).with_context(|| format!("failed to write {}", path.display()))?;
    make_executable(&path)?;
    println!("✔ Hook set at {}", path.display());
    Ok(())
}

fn hook_unset(rt: &Runtime) -> Result<()> {
    git::assert_git_repo(&rt.cwd)?;
    let path = hook_path(rt)?;
    if !path.exists() {
        println!("OpenCommit wasn't previously set as '{HOOK_NAME}' hook, nothing to remove");
        return Ok(());
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    if !existing.contains(MANAGED_MARKER) {
        println!(
            "OpenCommit wasn't previously set as '{HOOK_NAME}' hook, but a different hook was. It was preserved; remove it manually if desired."
        );
        return Ok(());
    }
    fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
    println!("✔ Hook is removed");
    Ok(())
}

async fn hook_run(rt: &Runtime, args: &[String]) -> Result<()> {
    let Some(message_file) = args.first() else {
        bail!(
            "Commit message file path is missing. This command should be called from the \"prepare-commit-msg\" git hook."
        );
    };
    if args
        .get(1)
        .map(|source| !source.is_empty())
        .unwrap_or(false)
    {
        return Ok(());
    }

    let config = rt.load_config()?;
    let staged = git::get_staged_files(&rt.cwd)?;
    if staged.is_empty() {
        return Ok(());
    }

    if requires_api_key(&config) && config.string_or("OCO_API_KEY", "").is_empty() {
        eprintln!(
            "No OCO_API_KEY is set. Set your key via `oco config set OCO_API_KEY=<value>`. Skipping the OpenCommit hook."
        );
        return Ok(());
    }

    let diff = git::get_diff(&rt.cwd, &staged)?;
    if diff.trim().is_empty() {
        return Ok(());
    }

    let message = commit::generate_message(rt, &config, &diff, false, "").await?;
    let file_content = fs::read_to_string(message_file).unwrap_or_default();

    let merged = if config.bool_or("OCO_HOOK_AUTO_UNCOMMENT", false) {
        format!("{message}\n\n{file_content}")
    } else {
        format!(
            "# {message}\n\n# ---------- [OpenCommit] ---------- #\n# Remove the # above to use this generated commit message.\n# To cancel the commit, just close this window without making any changes.\n\n{file_content}"
        )
    };

    fs::write(message_file, merged).with_context(|| format!("failed to write {message_file}"))?;
    Ok(())
}

fn requires_api_key(config: &Config) -> bool {
    let provider = config.provider().to_ascii_lowercase();
    !(provider.starts_with("ollama") || provider.starts_with("mlx") || provider == "test")
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

fn print_hook_help() {
    println!(
        "Manage the OpenCommit prepare-commit-msg hook\n\
         \n\
         Usage: oco hook <set|unset>\n\
         \n\
         `oco hook run <message-file> [source]` is used by the managed hook."
    );
}
