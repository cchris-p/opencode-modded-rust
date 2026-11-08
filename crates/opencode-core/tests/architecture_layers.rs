//! Enforces the stable-core / boundary / policy dependency rule.
//!
//! `invariants/future-proofing.md` requires that the stable core never imports a
//! volatile policy crate. That rule is checked here rather than by convention, so
//! a new core -> policy edge fails `cargo test` before it can land.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    /// Authority and durability: session/task state contracts, lifecycle, tools,
    /// permissions, storage, provider abstraction, event bus, decision boundary.
    Core,
    /// Adapters that wire the core to policy or vendors.
    Boundary,
    /// Volatile, replaceable policy (optional capabilities, prompts, engines).
    Policy,
    /// Process entry points (CLI, TUI) that may wire everything together.
    Interface,
}

/// The boundary map for every workspace crate.
///
/// Adding a crate requires adding it here; adding or removing a decision class,
/// shim, prompt, or profile does not.
const CRATE_LAYERS: &[(&str, Layer)] = &[
    ("opencode-core", Layer::Core),
    ("opencode-types", Layer::Core),
    ("opencode-util", Layer::Core),
    ("opencode-storage", Layer::Core),
    ("opencode-permission", Layer::Core),
    ("opencode-plugin", Layer::Core),
    ("opencode-retrieval", Layer::Core),
    ("opencode-provider", Layer::Boundary),
    ("opencode-tool", Layer::Boundary),
    ("opencode-grep", Layer::Boundary),
    ("opencode-lsp", Layer::Boundary),
    ("opencode-mcp", Layer::Boundary),
    ("opencode-config", Layer::Boundary),
    ("opencode-command", Layer::Boundary),
    ("opencode-agent", Layer::Boundary),
    ("opencode-session", Layer::Boundary),
    ("opencode-server", Layer::Boundary),
    ("opencode-watcher", Layer::Boundary),
    ("opencode-scopemux", Layer::Policy),
    ("opencode-cli", Layer::Interface),
    ("opencode-tui", Layer::Interface),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("core crate lives under <workspace>/crates/opencode-core")
        .to_path_buf()
}

/// Normal `[dependencies]` entries only. Dev-dependencies may use test helpers
/// from any layer without weakening the runtime rule.
fn opencode_dependencies(crate_dir: &Path) -> Vec<String> {
    let manifest = fs::read_to_string(crate_dir.join("Cargo.toml"))
        .unwrap_or_else(|error| panic!("cannot read {}/Cargo.toml: {error}", crate_dir.display()));

    let mut in_dependencies = false;
    let mut deps = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
            continue;
        }
        if !in_dependencies {
            continue;
        }
        let Some((name, _)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.starts_with("opencode-") {
            deps.push(name.to_string());
        }
    }
    deps
}

fn layer_map() -> BTreeMap<&'static str, Layer> {
    CRATE_LAYERS.iter().copied().collect()
}

#[test]
fn every_workspace_crate_is_categorized() {
    let map = layer_map();
    let crates_dir = workspace_root().join("crates");
    let mut found = Vec::new();

    for entry in fs::read_dir(&crates_dir).expect("crates directory should exist") {
        let path = entry.expect("crate entry").path();
        if !path.join("Cargo.toml").is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("crate name should be utf-8")
            .to_string();
        assert!(
            map.contains_key(name.as_str()),
            "crate `{name}` is missing from the architectural boundary map in \
             crates/opencode-core/tests/architecture_layers.rs"
        );
        found.push(name);
    }

    for name in map.keys() {
        assert!(
            found.iter().any(|found| found == name),
            "boundary map lists `{name}`, but no such crate exists"
        );
    }
}

#[test]
fn stable_core_only_depends_on_stable_core() {
    let map = layer_map();
    let root = workspace_root();

    for (name, layer) in CRATE_LAYERS {
        if *layer != Layer::Core {
            continue;
        }
        for dep in opencode_dependencies(&root.join("crates").join(name)) {
            let dep_layer = map.get(dep.as_str()).unwrap_or_else(|| {
                panic!("core crate `{name}` depends on unclassified crate `{dep}`")
            });
            assert_eq!(
                *dep_layer,
                Layer::Core,
                "stable core `{name}` must not depend on `{dep}` ({dep_layer:?}); \
                 put the interface behind the decision boundary instead"
            );
        }
    }
}

#[test]
fn policy_only_depends_on_stable_core() {
    let map = layer_map();
    let root = workspace_root();

    for (name, layer) in CRATE_LAYERS {
        if *layer != Layer::Policy {
            continue;
        }
        for dep in opencode_dependencies(&root.join("crates").join(name)) {
            let dep_layer = map
                .get(dep.as_str())
                .unwrap_or_else(|| panic!("policy crate `{name}` depends on unclassified `{dep}`"));
            assert_eq!(
                *dep_layer,
                Layer::Core,
                "policy `{name}` may depend only on core contracts, not on `{dep}` ({dep_layer:?})"
            );
        }
    }
}

#[test]
fn boundary_crates_do_not_depend_on_process_interfaces() {
    let map = layer_map();
    let root = workspace_root();

    for (name, layer) in CRATE_LAYERS {
        if *layer != Layer::Boundary {
            continue;
        }
        for dep in opencode_dependencies(&root.join("crates").join(name)) {
            let dep_layer = map.get(dep.as_str()).unwrap_or_else(|| {
                panic!("boundary crate `{name}` depends on unclassified `{dep}`")
            });
            assert_ne!(
                *dep_layer,
                Layer::Interface,
                "boundary `{name}` must not depend on process interface `{dep}`"
            );
        }
    }
}

#[test]
fn core_has_no_decision_engine_dependency() {
    let root = workspace_root();
    let core_deps = opencode_dependencies(&root.join("crates/opencode-core"));
    for dep in core_deps {
        for forbidden in ["jev", "laya", "scopemux"] {
            assert!(
                !dep.contains(forbidden),
                "core must not depend on engine/policy crate `{dep}`"
            );
        }
    }
}
