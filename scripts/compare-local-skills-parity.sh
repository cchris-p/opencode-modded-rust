#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/compare-local-skills-parity.sh [workspace]

Compares locally discovered skill names between vanilla OpenCode and OpenCode Rust
for the same workspace and user environment. URL-backed skills are outside this
check's scope.

Environment overrides:
  VANILLA_OPENCODE_DIR   Path to vanilla OpenCode checkout. Default: $HOME/repos/opencode-modded
  VANILLA_OPENCODE_CMD   Command run from the workspace. Default: bun $VANILLA_OPENCODE_DIR/packages/opencode/src/index.ts debug skill
  RUST_OPENCODE_CMD      Command run from the workspace. Default: cargo run -q --manifest-path <rust>/Cargo.toml -p opencode-cli -- debug skill
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

workspace="${1:-$PWD}"
repo_root="$(git rev-parse --show-toplevel)"
vanilla_dir="${VANILLA_OPENCODE_DIR:-$HOME/repos/opencode-modded}"
vanilla_cmd="${VANILLA_OPENCODE_CMD:-bun '$vanilla_dir/packages/opencode/src/index.ts' debug skill}"
rust_cmd="${RUST_OPENCODE_CMD:-cargo run -q --manifest-path '$repo_root/Cargo.toml' -p opencode-cli -- debug skill}"

if [[ ! -d "$workspace" ]]; then
  printf 'Workspace does not exist: %s\n' "$workspace" >&2
  exit 2
fi

if [[ ! -d "$vanilla_dir" ]]; then
  printf 'Vanilla OpenCode checkout does not exist: %s\n' "$vanilla_dir" >&2
  exit 2
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

run_json_names() {
  local command=$1
  local output=$2

  (
    cd "$workspace"
    PATH="$vanilla_dir/node_modules/.bin:$repo_root/target/debug:$PATH" bash -lc "$command"
  ) > "$output"

  python3 - "$output" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as f:
    data = json.load(f)

for name in sorted({item["name"] for item in data if item.get("location") != "<built-in>"}):
    print(name)
PY
}

run_json_names "$vanilla_cmd" "$tmpdir/vanilla.json" > "$tmpdir/vanilla.names"
run_json_names "$rust_cmd" "$tmpdir/rust.json" > "$tmpdir/rust.names"

if diff -u "$tmpdir/vanilla.names" "$tmpdir/rust.names"; then
  printf 'Local skill name lists match.\n'
else
  printf 'Local skill name lists differ.\n' >&2
  exit 1
fi
