#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/compare-openai-model-parity.sh

Verifies that the Rust product's user-facing OpenAI model list matches the
canonical vanilla OpenCode models.dev catalog.

The expected list is derived with the same rules vanilla applies in
`fromModelsDevProvider` / provider loading: keep non-deprecated, non-alpha base
models, then add one model per `experimental.modes` entry (`<id>-<mode>`).

The Rust CLI is run against a temporary cache seeded with the exact fetched
catalog so the comparison is deterministic and also exercises the models.dev
parse path.

Environment overrides:
  MODELS_URL             Catalog JSON URL. Default: https://models.opencode.ai/api.json
  RUST_OPENCODE_BIN      Rust CLI binary. Default: <repo>/target/debug/opencode
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel)"
models_url="${MODELS_URL:-https://models.opencode.ai/api.json}"
rust_bin="${RUST_OPENCODE_BIN:-$repo_root/target/debug/opencode}"

if [[ ! -x "$rust_bin" ]]; then
  printf 'Rust binary not found or not executable: %s\n' "$rust_bin" >&2
  printf 'Build it first with `ort-build` or `cargo build -p opencode-cli`.\n' >&2
  exit 2
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

catalog="$tmpdir/api.json"
curl -fsSL "$models_url" -o "$catalog"

mkdir -p "$tmpdir/cache/opencode"
cp "$catalog" "$tmpdir/cache/opencode/models.json"

python3 - "$catalog" > "$tmpdir/vanilla.ids" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as f:
    catalog = json.load(f)

models = catalog.get("openai", {}).get("models", {})
ids = set()
for model_id, model in models.items():
    status = model.get("status")
    if status in ("deprecated", "alpha"):
        continue
    ids.add(model_id)
    for mode in (model.get("experimental") or {}).get("modes", {}) or {}:
        ids.add(f"{model_id}-{mode}")

for model_id in sorted(ids):
    print(model_id)
PY

(
  XDG_CACHE_HOME="$tmpdir/cache" \
  OPENAI_API_KEY="model-parity-check" \
  "$rust_bin" models openai 2>/dev/null
) | awk '
  /^Provider: OpenAI \(openai\)/ { in_provider = 1; next }
  /^Provider:/ { in_provider = 0 }
  in_provider && /^  [^ ]/ { sub(/^  /, ""); print }
' | LC_ALL=C sort -u > "$tmpdir/rust.ids"

if diff -u "$tmpdir/vanilla.ids" "$tmpdir/rust.ids"; then
  printf 'OpenAI model list matches vanilla models.dev catalog (%s models).\n' \
    "$(wc -l < "$tmpdir/vanilla.ids")"
else
  printf 'OpenAI model lists differ.\n' >&2
  exit 1
fi
