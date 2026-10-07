//! Build script for the native `scopemux-core` FFI provider.
//!
//! When the `native` feature is enabled this configures and builds the
//! `scopemux-core` C library (`parser_core`) and links it, along with the
//! vendored Tree-sitter static libraries, into the crate.
//!
//! The source tree is taken from `SCOPEMUX_CORE_DIR`, falling back to
//! `third_party/scopemux-core` in this repository. Use
//! `scripts/fetch-scopemux-core.sh` to populate that directory at the pinned
//! revision.
//!
//! Without the `native` feature this script does nothing, so default builds
//! never require CMake or a C toolchain.

use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SCOPEMUX_CORE_DIR");
    println!("cargo:rerun-if-env-changed=SCOPEMUX_SKIP_NATIVE_BUILD");

    if env::var_os("CARGO_FEATURE_NATIVE").is_none() {
        return;
    }

    // Dev aid: compile the Rust FFI surface without building the C library
    // (`cargo check` does not link, so unresolved symbols are not an issue).
    if env::var_os("SCOPEMUX_SKIP_NATIVE_BUILD").is_some() {
        println!("cargo:warning=SCOPEMUX_SKIP_NATIVE_BUILD set; skipping scopemux-core build");
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate is nested under the workspace root")
        .to_path_buf();

    let source_dir = env::var("SCOPEMUX_CORE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("third_party/scopemux-core"));

    if !source_dir.join("CMakeLists.txt").is_file() {
        panic!(
            "scopemux-core sources not found at {}.\n\
             Set SCOPEMUX_CORE_DIR or run scripts/fetch-scopemux-core.sh.",
            source_dir.display()
        );
    }

    // The C build compiles the Tree-sitter grammars and pybind11 from submodules
    // of scopemux-core. A checkout that is missing them makes CMake fail later
    // with an opaque "No download info given ... is not an existing non-empty
    // directory" error. Repair the submodules here, before CMake, so any build
    // invocation (ort-build, cargo, CI) self-heals instead of failing.
    ensure_submodules(&source_dir);

    println!("cargo:rerun-if-changed={}", source_dir.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Scope the CMake build tree to the resolved source path. The same OUT_DIR is
    // reused when SCOPEMUX_CORE_DIR changes (for example switching between a local
    // checkout and third_party/scopemux-core), and reusing a CMake cache generated
    // for a different source directory fails configuration.
    let source_tag = {
        let mut hasher = DefaultHasher::new();
        source_dir.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };
    let build_dir = out_dir.join(format!("scopemux-core-build-{source_tag}"));

    // scopemux-core is developed and CI-tested with GCC, where implicit
    // function declarations and some pointer-type mismatches are warnings.
    // AppleClang treats them as errors, so relax just those diagnostics for the
    // native C build.
    let relaxed_c_flags = "-Wno-implicit-function-declaration \
         -Wno-incompatible-function-pointer-types \
         -Wno-incompatible-pointer-types \
         -Wno-int-conversion";

    // scopemux-core's CMake requires Python 3.10 or 3.11 (its range is
    // `3.10...<3.12`). The interpreter on PATH is often newer (a pyenv 3.12
    // virtualenv, a distro 3.12+ default), which CMake rejects and then
    // replaces with a system Python that lacks the development package. Resolve
    // a compatible interpreter here and pass it explicitly so the native build
    // does not depend on PATH ordering or pyenv state.
    let python_interpreter = resolve_python_interpreter();

    let mut configure = Command::new("cmake");
    configure
        .arg("-S")
        .arg(&source_dir)
        .arg("-B")
        .arg(&build_dir)
        .arg("-DSCOPEMUX_BUILD_TESTS=OFF")
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!("-DCMAKE_C_FLAGS={relaxed_c_flags}"));
    if let Some(python) = &python_interpreter {
        configure.arg(format!("-DPython_EXECUTABLE={}", python.display()));
    }

    let configure = configure
        .status()
        .expect("failed to run cmake (is it installed?)");
    assert!(
        configure.success(),
        "scopemux-core CMake configuration failed.\n\
         Common causes: missing Tree-sitter/pybind11 submodules (run \
         scripts/fetch-scopemux-core.sh) or a Python without 3.10/3.11 \
         development headers (set SCOPEMUX_PYTHON to a compatible interpreter)."
    );

    let build = Command::new("cmake")
        .arg("--build")
        .arg(&build_dir)
        .arg("--target")
        .arg("parser_core")
        .status()
        .expect("failed to run cmake --build");
    assert!(build.success(), "scopemux-core parser_core build failed");

    // scopemux-core resolves Tree-sitter query files from SCMU_QUERIES_DIR at
    // runtime (see `lib.rs`). Copy the source queries into OUT_DIR and bake that
    // path, so the built binary does not depend on the source checkout still
    // existing (important for worktrees and for re-fetching the core).
    let queries_src = source_dir.join("queries");
    println!("cargo:rerun-if-changed={}", queries_src.display());
    let queries_dst = out_dir.join("queries");
    if queries_src.is_dir() {
        let _ = std::fs::remove_dir_all(&queries_dst);
        copy_dir(&queries_src, &queries_dst).expect("failed to copy scopemux-core queries");
        println!(
            "cargo:rustc-env=SCOPEMUX_QUERIES_DIR={}",
            queries_dst.display()
        );
    } else {
        println!(
            "cargo:rustc-env=SCOPEMUX_QUERIES_DIR={}",
            queries_src.display()
        );
    }

    // Static libraries live in the build tree and the Tree-sitter library dir.
    println!(
        "cargo:rustc-link-search=native={}",
        build_dir.join("core").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        build_dir.join("tree-sitter-libs").display()
    );

    // The tree-sitter C runtime is intentionally not linked here. The product
    // already depends on the `tree-sitter` crate (via `opencode-tool`), and
    // linking scopemux-core's separate copy produces duplicate `ts_*` symbols
    // (a hard link error with GNU ld/rust-lld). The crate's runtime fulfils
    // parser_core and the grammar archives; the `native` feature depends on it
    // for standalone builds. Only the grammar archives and parser_core are
    // linked from the scopemux-core build tree.
    for lib in [
        "parser_core",
        "tree-sitter-c",
        "tree-sitter-cpp",
        "tree-sitter-python",
        "tree-sitter-javascript",
        "tree-sitter-typescript",
        "tree-sitter-rust",
    ] {
        println!("cargo:rustc-link-lib=static={lib}");
    }

    // System libraries the C core depends on.
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=m");
    }
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}

/// Submodule paths of `scopemux-core` the native C build compiles from source.
/// CMake's `ExternalProject_Add` points at `vendor/tree-sitter` (and the grammar
/// archives) directly, so an empty checkout fails configuration with
/// "No download info given ... is not an existing non-empty directory".
const REQUIRED_SUBMODULES: &[&str] = &[
    "vendor/tree-sitter",
    "vendor/tree-sitter-c",
    "vendor/tree-sitter-cpp",
    "vendor/tree-sitter-python",
    "vendor/tree-sitter-javascript",
    "vendor/tree-sitter-typescript",
    "vendor/tree-sitter-rust",
    "vendor/pybind11",
];

/// Ensure every [`REQUIRED_SUBMODULES`] entry under `source_dir` is checked out.
/// When any is missing or uninitialized and `source_dir` is a git checkout,
/// initialize the submodules in place; otherwise fail with an actionable error
/// instead of a cryptic CMake message.
fn ensure_submodules(source_dir: &Path) {
    let missing = || -> Vec<&'static str> {
        REQUIRED_SUBMODULES
            .iter()
            .copied()
            .filter(|rel| submodule_needs_init(source_dir, rel))
            .collect()
    };

    let initially_missing = missing();
    if initially_missing.is_empty() {
        return;
    }

    if source_dir.join(".git").exists() {
        // Limit to the submodules the C build needs: the test-case submodules are
        // private and are not required here (SCOPEMUX_BUILD_TESTS=OFF), so
        // initializing only the public ones keeps this working on machines that
        // have no GitHub credentials for the private repos.
        let mut update = Command::new("git");
        update.arg("-C").arg(source_dir).args([
            "submodule",
            "update",
            "--init",
            "--recursive",
            "--",
        ]);
        for rel in REQUIRED_SUBMODULES {
            update.arg(rel);
        }
        let status = update.status();
        if matches!(status, Ok(status) if status.success()) {
            let still_missing = missing();
            if still_missing.is_empty() {
                println!(
                    "cargo:warning=initialized missing scopemux-core submodules: {}",
                    initially_missing.join(", ")
                );
                return;
            }
            panic!(
                "scopemux-core submodules are still missing or uninitialized after \
                 `git submodule update --init --recursive`: {}.\n\
                 Run scripts/fetch-scopemux-core.sh to provision them.",
                still_missing.join(", ")
            );
        }
    }

    panic!(
        "scopemux-core is missing required submodule sources: {}.\n\
         The C build compiles these Tree-sitter grammars (and pybind11) from source.\n\
         Run scripts/fetch-scopemux-core.sh, or set SCOPEMUX_CORE_DIR to a fully \
         checked-out scopemux-core.",
        initially_missing.join(", ")
    );
}

/// Whether `path` is an existing, non-empty directory.
fn dir_is_nonempty(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

/// Whether the required submodule at `rel` is absent, empty, or uninitialized.
///
/// A merely existing directory is not enough: a failed or partial clone can
/// leave an empty directory or a bare `.git` gitlink with no sources, which
/// passes a plain "is it non-empty" test but leaves CMake unable to configure.
/// When `source_dir` is a git checkout, ask git for the real submodule state.
fn submodule_needs_init(source_dir: &Path, rel: &str) -> bool {
    if !dir_is_nonempty(&source_dir.join(rel)) {
        return true;
    }
    matches!(
        submodule_state(source_dir, rel),
        Some(SubmoduleState::Uninitialized | SubmoduleState::Conflict)
    )
}

#[derive(Clone, Copy)]
enum SubmoduleState {
    /// Checked out at the commit recorded by the superproject.
    Initialized,
    /// Checked out at a different commit.
    Modified,
    /// Registered but not checked out (empty directory or bare gitlink).
    Uninitialized,
    /// Merge conflict inside the submodule.
    Conflict,
}

/// Parse `git submodule status -- <rel>` into a [`SubmoduleState`].
///
/// Returns `None` when `source_dir` is not a git checkout, git is unavailable,
/// or `rel` is not a registered submodule, so callers fall back to the plain
/// directory check.
fn submodule_state(source_dir: &Path, rel: &str) -> Option<SubmoduleState> {
    if !source_dir.join(".git").exists() {
        return None;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(source_dir)
        .args(["submodule", "status", "--"])
        .arg(rel)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    match stdout.lines().next()?.chars().next()? {
        ' ' => Some(SubmoduleState::Initialized),
        '+' => Some(SubmoduleState::Modified),
        '-' => Some(SubmoduleState::Uninitialized),
        'U' => Some(SubmoduleState::Conflict),
        _ => None,
    }
}

/// Python program that exits 0 only when the interpreter it runs under is in
/// scopemux-core's supported range and has the development files CMake needs.
const PYTHON_PROBE: &str = r#"
import glob, os, sys, sysconfig

if not ((3, 10) <= sys.version_info[:2] < (3, 12)):
    raise SystemExit(1)
include = sysconfig.get_path("include")
if not include or not os.path.exists(os.path.join(include, "Python.h")):
    raise SystemExit(1)
libdir = sysconfig.get_config_var("LIBDIR")
if libdir and not glob.glob(os.path.join(libdir, "libpython*")):
    raise SystemExit(1)
"#;

/// Resolve a Python interpreter acceptable to scopemux-core's CMake.
///
/// `SCOPEMUX_PYTHON` wins when set. Otherwise this checks versioned binaries
/// first, then `python3`, then pyenv version directories (which carry dev
/// headers but are not always reachable as a bare `python3.11` on PATH).
fn resolve_python_interpreter() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(explicit) = env::var_os("SCOPEMUX_PYTHON") {
        candidates.push(PathBuf::from(explicit));
    }
    for name in ["python3.11", "python3.10", "python3"] {
        candidates.push(PathBuf::from(name));
    }

    let pyenv_root = env::var_os("PYENV_ROOT")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".pyenv")));
    if let Some(root) = pyenv_root {
        if let Ok(entries) = std::fs::read_dir(root.join("versions")) {
            let mut versions: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|dir| {
                    let name = dir.file_name().unwrap_or_default().to_string_lossy();
                    name.starts_with("3.11.") || name.starts_with("3.10.")
                })
                .collect();
            versions.sort();
            for dir in versions {
                candidates.push(dir.join("bin").join("python3"));
            }
        }
    }

    candidates
        .into_iter()
        .find(|candidate| python_is_usable(candidate))
}

/// Whether `python` satisfies [`PYTHON_PROBE`].
fn python_is_usable(python: &Path) -> bool {
    Command::new(python)
        .arg("-c")
        .arg(PYTHON_PROBE)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Recursively copy a directory tree, creating `dst` as needed.
fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
