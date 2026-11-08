use crate::{PermissionRequest, ToolContext, ToolError};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ExternalDirectoryOptions {
    pub bypass: bool,
    pub kind: ExternalDirectoryKind,
}

impl Default for ExternalDirectoryOptions {
    fn default() -> Self {
        Self {
            bypass: false,
            kind: ExternalDirectoryKind::File,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExternalDirectoryKind {
    File,
    Directory,
}

pub async fn assert_external_directory(
    ctx: &ToolContext,
    target: Option<&str>,
    options: ExternalDirectoryOptions,
) -> Result<(), ToolError> {
    let target = match target {
        Some(t) => t,
        None => return Ok(()),
    };

    if options.bypass {
        return Ok(());
    }

    if is_within_project(target, &ctx.project_root) {
        return Ok(());
    }

    ctx.ask_permission(permission_request(target, options.kind))
        .await
}

fn is_within_project(target: &str, project_root: &str) -> bool {
    let target_path = Path::new(target);

    if target_path.is_absolute() {
        return target_path.starts_with(project_root);
    }

    if target.starts_with("./") || target.starts_with("../") {
        return true;
    }

    if !target.starts_with('/') && !target.contains(':') {
        return true;
    }

    false
}

pub fn get_parent_directory(target: &str, kind: ExternalDirectoryKind) -> String {
    get_directory_boundary(target, kind)
}

pub fn get_directory_boundary(target: &str, kind: ExternalDirectoryKind) -> String {
    match kind {
        ExternalDirectoryKind::Directory => canonical_directory_boundary(Path::new(target)),
        ExternalDirectoryKind::File => {
            let path = Path::new(target);
            if let Ok(canonical) = std::fs::canonicalize(path) {
                if let Some(parent) = canonical.parent() {
                    return normalize_directory_path(parent);
                }
            }

            let parent = path.parent().unwrap_or(path);
            canonical_directory_boundary(parent)
        }
    }
}

pub fn make_glob_pattern(parent_dir: &str) -> String {
    let parent_dir = normalize_directory_path(Path::new(parent_dir));
    if parent_dir == "/" || parent_dir.ends_with(':') {
        format!("{parent_dir}*")
    } else {
        format!("{parent_dir}/*")
    }
}

pub fn permission_request(target: &str, kind: ExternalDirectoryKind) -> PermissionRequest {
    let parent_dir = get_directory_boundary(target, kind);
    PermissionRequest::new("external_directory")
        .with_pattern(make_glob_pattern(&parent_dir))
        .with_metadata("filepath", serde_json::json!(target))
        .with_metadata("parentDir", serde_json::json!(parent_dir))
}

fn canonical_directory_boundary(path: &Path) -> String {
    let mut current = PathBuf::from(path);
    loop {
        if let Ok(canonical) = std::fs::canonicalize(&current) {
            let directory = if canonical.is_dir() {
                canonical
            } else {
                canonical
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or(canonical)
            };
            return normalize_directory_path(&directory);
        }

        let Some(parent) = current.parent() else {
            return normalize_directory_path(path);
        };

        if parent == current {
            return normalize_directory_path(path);
        }

        current = parent.to_path_buf();
    }
}

fn normalize_directory_path(path: &Path) -> String {
    let rendered = path.to_string_lossy();
    let trimmed = rendered.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        rendered.into_owned()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_is_within_project_relative() {
        assert!(is_within_project("./src/main.rs", "/home/user/project"));
        assert!(is_within_project("../other/file.txt", "/home/user/project"));
        assert!(is_within_project("src/lib.rs", "/home/user/project"));
    }

    #[test]
    fn test_is_within_project_absolute() {
        assert!(is_within_project(
            "/home/user/project/src/main.rs",
            "/home/user/project"
        ));
        assert!(!is_within_project(
            "/home/other/file.txt",
            "/home/user/project"
        ));
    }

    #[test]
    fn test_get_parent_directory_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, "demo").unwrap();
        let canonical_dir = fs::canonicalize(dir.path()).unwrap();

        let parent = get_parent_directory(&file.to_string_lossy(), ExternalDirectoryKind::File);
        assert_eq!(parent, canonical_dir.to_string_lossy());
    }

    #[test]
    fn test_get_parent_directory_dir() {
        let dir = tempfile::tempdir().unwrap();
        let canonical_dir = fs::canonicalize(dir.path()).unwrap();

        let parent = get_parent_directory(
            &dir.path().to_string_lossy(),
            ExternalDirectoryKind::Directory,
        );
        assert_eq!(parent, canonical_dir.to_string_lossy());
    }

    #[test]
    fn test_make_glob_pattern() {
        let pattern = make_glob_pattern("/home/user/external");
        assert_eq!(pattern, "/home/user/external/*");
    }

    #[test]
    fn test_make_glob_pattern_trims_trailing_separator() {
        let pattern = make_glob_pattern("/home/user/external/");
        assert_eq!(pattern, "/home/user/external/*");
    }

    #[test]
    fn test_permission_request_uses_directory_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        let canonical_dir = fs::canonicalize(dir.path()).unwrap();

        let request = permission_request(&file.to_string_lossy(), ExternalDirectoryKind::File);
        assert_eq!(
            request.patterns,
            vec![format!("{}/*", canonical_dir.to_string_lossy())]
        );
    }
}
