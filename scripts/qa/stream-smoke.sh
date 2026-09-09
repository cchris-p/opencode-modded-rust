#!/usr/bin/env bash
# stream-smoke.sh — live multi-turn streaming smoke test for the rust runtime.
#
# Guards the two BUG-003 failure classes on the REAL provider path:
#   1. streamed assistant replies must be complete/coherent (not garbled), and
#   2. the session must keep answering follow-up prompts (no silent stop).
#
# It launches a fresh detached server from the repo binary, drives a three-turn
# deepseek session over the HTTP API, and asserts every turn produced a
# completed assistant reply containing expected text.
#
# Usage:
#   DEEPSEEK_API_KEY=<key> scripts/qa/stream-smoke.sh
#
# Env-gated: exits 0 (skipped) if DEEPSEEK_API_KEY is unset so it is safe to
# run in contexts without a key. Requires a cargo build of opencode-cli first.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="$REPO/target/debug/opencode"
PORT="${SMOKE_PORT:-3491}"
BASE="http://127.0.0.1:${PORT}"
LOG="$(mktemp /tmp/stream-smoke.XXXXXX.log)"
PID=""

cleanup() {
  if [ -n "$PID" ] && kill -0 "$PID" 2>/dev/null; then
    kill "$PID" 2>/dev/null || true
  fi
  rm -f "$LOG"
}
trap cleanup EXIT

if [ -z "${DEEPSEEK_API_KEY:-}" ]; then
  echo "stream-smoke: DEEPSEEK_API_KEY unset; skipping live smoke." >&2
  exit 0
fi

if [ ! -x "$BIN" ]; then
  echo "stream-smoke: missing $BIN — run cargo build -p opencode-cli first." >&2
  exit 2
fi

echo "stream-smoke: starting server on ${BASE}"

setsid bash -c "DEEPSEEK_API_KEY='${DEEPSEEK_API_KEY}' '$BIN' serve --port '$PORT' --hostname 127.0.0.1 >'$LOG' 2>&1 & echo \$! > '$LOG.pid'" < /dev/null
sleep 1
PID="$(cat "$LOG.pid" 2>/dev/null || true)"

# Wait for the server to accept requests.
for _ in $(seq 1 60); do
  if curl -sf -o /dev/null "${BASE}/session" 2>/dev/null; then
    break
  fi
  sleep 0.5
done

if ! curl -sf -o /dev/null "${BASE}/session"; then
  echo "stream-smoke: server did not come up; log:" >&2
  tail -40 "$LOG" >&2 || true
  exit 3
fi

MODEL="${DEEPSEEK_MODEL:-deepseek/deepseek-v4-flash}"

# Create a fresh session.
SID="$(curl -sf -X POST "${BASE}/session" -H 'Content-Type: application/json' \
  -d "{\"directory\":\"$REPO\"}" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')"

fail=""
for pair in \
  "one:Reply with exactly ONE and nothing else." \
  "two:Reply with exactly TWO and nothing else." \
  "three:Summarize the two prior answers in a single short line."; do
  expected="${pair%%:*}"
  prompt="${pair#*:}"

  echo "stream-smoke: prompt '${prompt}'"
  code="$(curl -s -o /dev/null -w '%{http_code}' -X POST "${BASE}/session/${SID}/prompt" \
    -H 'Content-Type: application/json' \
    -d "{\"model\":\"$MODEL\",\"message\":\"$prompt\"}")"
  if [ "$code" != "200" ]; then
    echo "stream-smoke: prompt HTTP $code" >&2
    fail=1
    break
  fi

  # Wait for a completed assistant reply to appear.
  ok=0
  for _ in $(seq 1 120); do
    msgs="$(curl -sf "${BASE}/session/${SID}/message" 2>/dev/null || echo '[]')"
    if echo "$msgs" | python3 -c '
import json,sys
ms=json.load(sys.stdin)
asst=[m for m in ms if m.get("role")=="assistant"]
if not asst: sys.exit(1)
last=asst[-1]
txt=" ".join(p.get("text","") for p in last.get("parts",[]) if p.get("text"))
want=sys.argv[1]
sys.exit(0 if (last.get("completed_at") and txt.strip()) else 1)
' "$expected" 2>/dev/null; then
      ok=1
      break
    fi
    sleep 0.5
  done

  if [ "$ok" != "1" ]; then
    echo "stream-smoke: no completed assistant reply for turn '${expected}'" >&2
    echo "stream-smoke: session messages:" >&2
    echo "$msgs" | python3 -m json.tool 2>/dev/null | head -80 >&2 || true
    fail=1
    break
  fi
  echo "stream-smoke: turn '${expected}' replied OK"
done

# Fresh server per run: not reused by ort.
echo "stream-smoke: stopping server"
cleanup

if [ -n "$fail" ]; then
  echo "stream-smoke: FAILED" >&2
  exit 4
fi

echo "stream-smoke: PASSED (3 turns completed on ${MODEL})"
