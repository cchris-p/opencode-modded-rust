//! Enforces the ScopeMux/LSP ownership boundary from `wiki/scopemux-lsp-boundary.md`.
//!
//! LSP owns exact point semantics and diagnostics. ScopeMux owns parsing, the map, and
//! token-budgeted context. This guard fails if a map-backed path (ScopeMux or the
//! retrieval boundary) starts reimplementing or proxying an LSP point query.
//!
//! It is intentionally a cheap source scan, not a runtime assertion: the goal is to stop
//! a duplicate surface from being introduced, not to exercise the map.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("core crate lives under <workspace>/crates/opencode-core")
        .to_path_buf()
}

fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("cannot read {}: {error}", dir.display());
    });
    for entry in entries {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            sources.extend(rust_sources(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            sources.push(path);
        }
    }
    sources
}

/// Tokens that mean a map-backed path is answering, proxying, or synthesizing an LSP
/// point query instead of leaving it to `opencode-lsp`.
const FORBIDDEN_IN_MAP_LAYER: &[&str] = &[
    "opencode_lsp",
    "opencode-lsp",
    "lsp_types",
    "textDocument/definition",
    "textDocument/references",
    "textDocument/hover",
    "textDocument/documentSymbol",
    "textDocument/implementation",
    "textDocument/typeDefinition",
    "textDocument/prepareCallHierarchy",
    "callHierarchy/incomingCalls",
    "callHierarchy/outgoingCalls",
    "publishDiagnostics",
    "goto_definition",
    "document_symbol",
    "type_definition",
    "prepare_call_hierarchy",
];

#[test]
fn scopemux_does_not_depend_on_the_lsp_client() {
    let manifest = fs::read_to_string(workspace_root().join("crates/opencode-scopemux/Cargo.toml"))
        .expect("scopemux manifest should exist");
    assert!(
        !manifest.contains("opencode-lsp"),
        "opencode-scopemux must not depend on opencode-lsp; LSP is a separate capability \
         (wiki/scopemux-lsp-boundary.md)"
    );
    assert!(
        !manifest.contains("lsp-types"),
        "opencode-scopemux must not depend on lsp_types; consumers use neutral fact structs"
    );
}

#[test]
fn map_layer_has_no_lsp_point_query_or_diagnostics_path() {
    let root = workspace_root();
    let mut violations = Vec::new();

    for crate_dir in [
        "crates/opencode-scopemux/src",
        "crates/opencode-retrieval/src",
    ] {
        for source in rust_sources(&root.join(crate_dir)) {
            let text = fs::read_to_string(&source)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
            for token in FORBIDDEN_IN_MAP_LAYER {
                if text.contains(token) {
                    violations.push(format!("{} contains `{}`", source.display(), token));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "LSP point queries and diagnostics belong to opencode-lsp, not the map-backed \
         layers (wiki/scopemux-lsp-boundary.md):\n{}",
        violations.join("\n")
    );
}
