use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::{Command, Output};

pub fn run_git(cwd: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))
}

pub fn run_git_checked(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = run_git(cwd, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn assert_git_repo(cwd: &Path) -> Result<()> {
    let output = run_git(cwd, &["rev-parse"])?;
    if !output.status.success() {
        bail!("not a git repository (or any of the parent directories)");
    }
    Ok(())
}

pub fn get_staged_files(cwd: &Path) -> Result<Vec<String>> {
    let stdout = run_git_checked(cwd, &["diff", "--name-only", "--cached", "--relative"])?;
    let mut files = split_lines(&stdout);
    files.sort();
    Ok(files)
}

pub fn get_changed_files(cwd: &Path) -> Result<Vec<String>> {
    let modified = run_git_checked(cwd, &["ls-files", "--modified"])?;
    let others = run_git_checked(cwd, &["ls-files", "--others", "--exclude-standard"])?;
    let mut files = split_lines(&modified);
    files.extend(split_lines(&others));
    files.sort();
    Ok(files)
}

pub fn git_add(cwd: &Path, files: &[String]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    let mut args = vec!["add", "--"];
    args.extend(files.iter().map(String::as_str));
    run_git_checked(cwd, &args)?;
    Ok(())
}

pub fn get_diff(cwd: &Path, files: &[String]) -> Result<String> {
    let (diffable, excluded): (Vec<&String>, Vec<&String>) =
        files.iter().partition(|file| !is_excluded_from_diff(file));
    if !excluded.is_empty() {
        eprintln!(
            "Some files are excluded by default from 'git diff'. No commit message is generated from these files:\n{}",
            excluded
                .iter()
                .map(|file| file.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    let mut args = vec!["diff", "--staged", "--"];
    args.extend(diffable.iter().map(|file| file.as_str()));
    run_git_checked(cwd, &args)
}

pub fn get_remotes(cwd: &Path) -> Result<Vec<String>> {
    let stdout = run_git_checked(cwd, &["remote"])?;
    Ok(split_lines(&stdout))
}

pub fn has_upstream_branch(cwd: &Path) -> Result<bool> {
    let output = run_git(
        cwd,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )?;
    Ok(output.status.success())
}

pub fn current_branch(cwd: &Path) -> Result<String> {
    run_git_checked(cwd, &["branch", "--show-current"])
}

pub fn push(cwd: &Path, remote: Option<&str>, set_upstream: bool) -> Result<()> {
    let mut args: Vec<String> = vec!["push".to_string()];
    if let Some(remote) = remote {
        args.push(remote.to_string());
    }
    if set_upstream {
        args.push("--set-upstream".to_string());
        if remote.is_none() {
            args.push("origin".to_string());
        }
        // git expects the branch name after --set-upstream.
        args.push(current_branch(cwd)?);
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_git(cwd, &arg_refs)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git push failed: {}", stderr.trim());
    }
    Ok(())
}

pub fn commit(cwd: &Path, message: &str, extra_args: &[String]) -> Result<String> {
    let mut args = vec!["commit".to_string(), "-m".to_string(), message.to_string()];
    args.extend(extra_args.iter().cloned());
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_git(cwd, &arg_refs)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "git commit failed: {}{}",
            stderr.trim(),
            if stdout.trim().is_empty() {
                String::new()
            } else {
                format!("\n{}", stdout.trim())
            }
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn hooks_dir(cwd: &Path) -> Result<std::path::PathBuf> {
    if let Ok(path) = run_git_checked(cwd, &["rev-parse", "--git-path", "hooks"]) {
        let path = Path::new(&path);
        return Ok(if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        });
    }
    Ok(cwd.join(".git").join("hooks"))
}

fn is_excluded_from_diff(file: &str) -> bool {
    file.contains(".lock")
        || file.contains("-lock.")
        || file.contains(".svg")
        || file.contains(".png")
        || file.contains(".jpg")
        || file.contains(".jpeg")
        || file.contains(".webp")
        || file.contains(".gif")
}

fn split_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}
