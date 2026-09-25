#!/usr/bin/env bash
#
# Boot a local OpenCode server for QA of the scope-scoped Scopemux provider.
#
# The native Scopemux provider only activates for the local Qwen-via-Ollama
# model (SCOPE-005). This helper starts the local Ollama instance and selects
# the shared `opencode-use-ollama-local` preset, then runs `opencode serve`
# from a small QA workspace so a TUI can attach and confirm the provider.
#
# Usage:
#   scripts/scopemux-qa-server.sh [--binary PATH] [--workspace DIR] [--port PORT]
#
# Environment overrides:
#   SCOPEMUX_QA_BINARY      opencode binary to run
#   SCOPEMUX_QA_WORKSPACE   workspace served (default: $HOME/worktrees/scopemux-qa)
#   SCOPEMUX_QA_PORT        port (default: 4096)
#   OPENCODE_RUST_REPO      product checkout (default: $HOME/repos/opencode-modded-rust)
#
# The shared standards aliases are sourced when present:
#   ~/standards/ollama-config   -> ollama-start-0
#   ~/standards/opencode-config -> opencode-use-ollama-local

# Note: no `-u`; the shared standards aliases are not nounset-safe.
set -eo pipefail

PORT="${SCOPEMUX_QA_PORT:-4096}"
WORKSPACE="${SCOPEMUX_QA_WORKSPACE:-$HOME/worktrees/scopemux-qa}"
REPO="${OPENCODE_RUST_REPO:-$HOME/repos/opencode-modded-rust}"
BINARY="${SCOPEMUX_QA_BINARY:-}"

usage() {
  sed -n '2,25p' "$0"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --binary) BINARY="${2:?--binary needs a path}"; shift 2 ;;
    --workspace) WORKSPACE="${2:?--workspace needs a path}"; shift 2 ;;
    --port) PORT="${2:?--port needs a port}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "scopemux-qa-server: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

# Resolve the binary: explicit override, then the main checkout, then the shared
# worktree build target used by this repository's worktrees.
if [ -z "$BINARY" ]; then
  for candidate in \
    "$REPO/target/debug/opencode" \
    "${OPENCODE_WORKTREE_ROOT:-$HOME/worktrees}/opencode-modded-rust/.shared-target/debug/opencode"
  do
    if [ -x "$candidate" ]; then
      BINARY="$candidate"
      break
    fi
  done
fi

if [ -z "$BINARY" ] || [ ! -x "$BINARY" ]; then
  echo "scopemux-qa-server: no opencode binary found; build one with ort-build or pass --binary" >&2
  exit 1
fi

# Bring up the local Ollama instance and force the local Qwen model preset.
# The shared standards files are not `set -e`/`set -u` safe, so relax strict
# mode while sourcing them, then restore it.
if [ -f "$HOME/standards/ollama-config" ]; then
  set +e
  # shellcheck disable=SC1090
  source "$HOME/standards/ollama-config"
  ollama-start-0
  set -e
fi

if [ -f "$HOME/standards/opencode-config" ]; then
  set +e
  # shellcheck disable=SC1090
  source "$HOME/standards/opencode-config"
  # Sourcing runs `opencode-use-auto`, which may pick a remote provider; force
  # the local Qwen preset the Scopemux provider is scoped to.
  opencode-use-ollama-local
  set -e
fi

# A workspace config keeps the model pinned for an attached TUI too; the rust
# product's `ort` path unsets OPENCODE_CONFIG_CONTENT, so the file is the
# durable source of the qwen model for this workspace.
mkdir -p "$WORKSPACE"
if [ ! -f "$WORKSPACE/opencode.json" ]; then
  printf '{\n  "model": "ollama/qwen3:30b"\n}\n' > "$WORKSPACE/opencode.json"
fi

# Stop a previous instance launched by this script on the same binary and port.
if existing="$(pgrep -f "^${BINARY} serve --port ${PORT}( |$)" 2>/dev/null)"; then
  echo "scopemux-qa-server: stopping previous server (pid ${existing})"
  # shellcheck disable=SC2086
  kill ${existing}
  sleep 1
fi

echo "scopemux-qa-server:"
echo "  binary:    $BINARY"
echo "  workspace: $WORKSPACE"
echo "  model:     ${OPENCODE_CONFIG_CONTENT:-<from $WORKSPACE/opencode.json>}"
echo "  url:       http://127.0.0.1:${PORT}"

cd "$WORKSPACE"
# Emit the retrieval-provider selection line consumed by scopemux-qa-check.sh.
export RUST_LOG="${RUST_LOG:-info}"
exec "$BINARY" serve --port "$PORT" --hostname 127.0.0.1
