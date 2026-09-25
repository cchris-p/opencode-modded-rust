#!/usr/bin/env bash
#
# Attach/detach QA check for the scoped Scopemux provider (SCOPE-005).
#
# Drives a running OpenCode server with a file-seeded prompt (the only path
# that triggers retrieval), reads the effective retrieval provider from the
# server log, and aborts the turn. This is the default agent-driven QA method
# for board items: attach a server, run the check, detach.
#
# The server must be started with `RUST_LOG=info` (the QA server helper does
# this) so the "retrieval provider selected" line is emitted.
#
# Usage:
#   scripts/scopemux-qa-check.sh [--url URL] [--workspace DIR] [--file REL]
#                               [--log PATH] [--expect PROVIDER]
#
# Exit 0 when the observed provider matches --expect (default: scopemux).

set -eo pipefail

URL="http://127.0.0.1:4096"
WORKSPACE="${SCOPEMUX_QA_WORKSPACE:-$HOME/worktrees/scopemux-qa}"
RELFILE="sample.rs"
LOG="${SCOPEMUX_QA_LOG:-/tmp/scopemux-qa-server.log}"
EXPECT="scopemux"

usage() { sed -n '2,20p' "$0"; }

while [ "$#" -gt 0 ]; do
  case "$1" in
    --url) URL="${2:?}"; shift 2 ;;
    --workspace) WORKSPACE="${2:?}"; shift 2 ;;
    --file) RELFILE="${2:?}"; shift 2 ;;
    --log) LOG="${2:?}"; shift 2 ;;
    --expect) EXPECT="${2:?}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "scopemux-qa-check: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

command -v jq >/dev/null || { echo "scopemux-qa-check: jq is required" >&2; exit 1; }

start_lines=0
[ -f "$LOG" ] && start_lines="$(wc -l < "$LOG" | tr -d ' ')"

sid="$(curl -sf -X POST "$URL/session?directory=$WORKSPACE" \
        -H 'content-type: application/json' -d '{}' | jq -r .id)"
[ -n "$sid" ] && [ "$sid" != "null" ] || { echo "scopemux-qa-check: failed to create session" >&2; exit 1; }

curl -sf -X POST "$URL/session/$sid/prompt_async" \
  -H 'content-type: application/json' \
  -d "$(jq -nc --arg m "QA scope check on @$RELFILE" '{message:$m}')" >/dev/null

observed=""
observed_candidates=""
for _ in $(seq 1 30); do
  sleep 1
  if [ -f "$LOG" ]; then
    line="$(tail -n +"$((start_lines + 1))" "$LOG" 2>/dev/null \
      | perl -pe 's/\e\[[0-9;]*[A-Za-z]//g' \
      | grep -a "retrieval provider selected" | tail -1)"
    if [ -n "$line" ]; then
      observed="$(printf '%s' "$line" | sed -n 's/.*provider=\([^ ]*\).*/\1/p')"
      observed_candidates="$(printf '%s' "$line" | sed -n 's/.*candidates=\([0-9]*\).*/\1/p')"
      break
    fi
  fi
done

# Detach: stop the running turn without touching the server.
curl -sf -X POST "$URL/session/$sid/prompt/abort" >/dev/null 2>&1 || true

echo "session:    $sid"
echo "file:       @$RELFILE"
echo "expected:   $EXPECT"
echo "observed:   ${observed:-<none>}"
echo "candidates: ${observed_candidates:-<none>}"

if [ "$observed" = "$EXPECT" ] && [ "${observed_candidates:-0}" -gt 0 ]; then
  echo "PASS"
  exit 0
fi

if [ "$observed" = "$EXPECT" ]; then
  echo "FAIL: provider matched but returned no candidates (workspace parse or indexing failed)"
  exit 1
fi

echo "FAIL"
exit 1
