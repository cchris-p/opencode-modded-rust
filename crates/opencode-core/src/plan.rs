//! Session-scoped plan file path resolution.
//!
//! Mirrors the reference `Session.plan`
//! (`packages/opencode/src/session/session.ts`): the plan file is
//! `<time.created>-<slug>.md` inside `<worktree>/.opencode/plans` for a VCS
//! project, or the product data `plans/` directory otherwise. This is the single
//! source of truth for the plan path; callers must not build the path ad hoc.

use std::path::{Path, PathBuf};

/// The product's local data directory (`dirs::data_local_dir()/opencode`).
///
/// macOS: `~/Library/Application Support/opencode`. Linux:
/// `~/.local/share/opencode` (or `$XDG_DATA_HOME/opencode`).
pub fn opencode_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("opencode"))
}

/// The directory that holds plan files for a project/worktree.
///
/// VCS projects keep plans inside the worktree at `.opencode/plans`; projects
/// without VCS fall back to the product data `plans/` directory.
pub fn plans_dir(worktree: &Path, global_data_dir: &Path) -> PathBuf {
    if worktree.join(".git").exists() {
        worktree.join(".opencode").join("plans")
    } else {
        global_data_dir.join("plans")
    }
}

/// Resolve the session-scoped, timestamped plan file path.
///
/// `created_ms` is the session's `time.created` in milliseconds.
pub fn plan_file_path(
    worktree: &Path,
    global_data_dir: &Path,
    slug: &str,
    created_ms: i64,
) -> PathBuf {
    plans_dir(worktree, global_data_dir).join(format!("{created_ms}-{slug}.md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "opencode-plan-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn vcs_project_uses_worktree_plans_dir() {
        let worktree = temp_dir("vcs");
        fs::create_dir_all(worktree.join(".git")).unwrap();
        let global = temp_dir("vcs-global");
        let path = plan_file_path(&worktree, &global, "slug", 1234);
        assert_eq!(
            path,
            worktree
                .join(".opencode")
                .join("plans")
                .join("1234-slug.md")
        );
    }

    #[test]
    fn non_vcs_project_uses_global_plans_dir() {
        let worktree = temp_dir("novcs");
        let global = temp_dir("novcs-global");
        let path = plan_file_path(&worktree, &global, "abc", 99);
        assert_eq!(path, global.join("plans").join("99-abc.md"));
    }

    #[test]
    fn git_worktree_file_marker_is_detected() {
        let worktree = temp_dir("wtfile");
        fs::write(worktree.join(".git"), "gitdir: /somewhere").unwrap();
        let global = temp_dir("wtfile-global");
        let path = plan_file_path(&worktree, &global, "s", 1);
        assert!(path.starts_with(worktree.join(".opencode").join("plans")));
    }

    #[test]
    fn distinct_sessions_get_distinct_paths() {
        let worktree = temp_dir("distinct");
        fs::create_dir_all(worktree.join(".git")).unwrap();
        let global = temp_dir("distinct-global");
        let a = plan_file_path(&worktree, &global, "one", 1);
        let b = plan_file_path(&worktree, &global, "two", 2);
        assert_ne!(a, b);
    }
}
