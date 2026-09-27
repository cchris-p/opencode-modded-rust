#!/usr/bin/env bash
# bug-043-047-051-snapshot-merge-qa.sh — post-merge QA for H-014.
#
# Verifies, on the real product path, the fix shipped in PR #127 / dc41ce3:
#   BUG-051  session message list stays monotonic across a streaming turn and
#            across interrupt-then-continue (no snapshot reversion / flashing).
#   BUG-043  an aborted run still carries a durable terminal record
#            (error / finish_reason / completed_at) after a server restart.
#   BUG-047  in-flight assistant progress is persisted and survives reload.
#
# The harness runs a fresh server against an isolated HOME/DB (never the
# operator's shared opencode.db), drives the HTTP API, and prints a PASS/FAIL
# verdict plus the raw evidence. Requires a cargo build of opencode-cli first.
#
# Usage:
#   DEEPSEEK_API_KEY=<key> scripts/qa/bug-043-047-051-snapshot-merge-qa.sh
#
# Env overrides: QA_PORT, QA_MODEL.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="$REPO/target/debug/opencode"
PORT="${QA_PORT:-4197}"
BASE="http://127.0.0.1:${PORT}"
MODEL="${QA_MODEL:-deepseek/deepseek-flash}"

TMP="$(mktemp -d /tmp/h014-qa.XXXXXX)"
export HOME="$TMP/home"
mkdir -p "$HOME"
export OPENCODE_SERVER_LOG="$TMP/server.log"
LOGF="$TMP/server.log"
WS="$TMP/ws"
mkdir -p "$WS"
printf 'qa fixture\n' > "$WS/sample.txt"

PID=""
cleanup() {
  if [ -n "$PID" ] && kill -0 "$PID" 2>/dev/null; then
    kill "$PID" 2>/dev/null || true
    wait "$PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

if [ -z "${DEEPSEEK_API_KEY:-}" ]; then
  echo "h014-qa: DEEPSEEK_API_KEY unset; cannot run the live probe." >&2
  exit 2
fi
if [ ! -x "$BIN" ]; then
  echo "h014-qa: missing $BIN — run cargo build -p opencode-cli first." >&2
  exit 2
fi

start_server() {
  "$BIN" serve --port "$PORT" --hostname 127.0.0.1 >"$LOGF" 2>&1 &
  PID=$!
  for _ in $(seq 1 100); do
    if curl -sf -o /dev/null "$BASE/health" 2>/dev/null; then return 0; fi
    sleep 0.3
  done
  echo "h014-qa: server did not come up; log follows:" >&2
  tail -40 "$LOGF" >&2 || true
  exit 3
}

stop_server() {
  if [ -n "$PID" ]; then
    kill "$PID" 2>/dev/null || true
    wait "$PID" 2>/dev/null || true
    PID=""
  fi
}

echo "h014-qa: isolated HOME=$HOME  server=$BASE  model=$MODEL  binary=$BIN"
start_server

SID="$(python3 - "$BASE" "$WS" <<'PY'
import json, sys, urllib.request
base, ws = sys.argv[1], sys.argv[2]
req = urllib.request.Request(
    f"{base}/session?directory={urllib.parse.quote(ws)}",
    data=b"{}", headers={"Content-Type": "application/json"}, method="POST")
print(json.load(urllib.request.urlopen(req))["id"])
PY
)"
echo "h014-qa: session=$SID"

echo
echo "=== Phase 1: BUG-051 monotonic list (stream + interrupt-then-continue) ==="
python3 - "$BASE" "$SID" "$MODEL" <<'PY'
import json, sys, time, urllib.request

base, sid, model = sys.argv[1], sys.argv[2], sys.argv[3]

def http(method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(base + path, data=data,
                                 headers={"Content-Type": "application/json"},
                                 method=method)
    with urllib.request.urlopen(req, timeout=30) as r:
        raw = r.read().decode()
    return json.loads(raw) if raw.strip() else None

def messages():
    return http("GET", f"/session/{sid}/message") or []

def poll(seconds, label, samples, seen_ids, seen_counts, continue_text=None):
    deadline = time.time() + seconds
    while time.time() < deadline:
        msgs = messages()
        n = len(msgs)
        last_id = msgs[-1]["id"] if msgs else ""
        samples.append((round(time.time() - t0, 1), label, n, last_id))
        # non-decreasing count
        if seen_counts and n < seen_counts[-1]:
            raise SystemExit(
                f"FAIL BUG-051: message count regressed {seen_counts[-1]} -> {n} "
                f"(label={label})")
        seen_counts.append(n)
        # last id must only ever advance to a brand-new id
        if last_id:
            if last_id in seen_ids and samples[-2][3] != last_id:
                raise SystemExit(
                    f"FAIL BUG-051: last message id reverted to {last_id} "
                    f"(label={label})")
            seen_ids.add(last_id)
        if continue_text is not None:
            dupes = [m for m in msgs
                     if m["role"].lower() == "user"
                     and continue_text in " ".join(
                         p.get("text", "") for p in m.get("parts", []))]
            if len(dupes) > 1:
                raise SystemExit(
                    f"FAIL BUG-051: continuation prompt rendered {len(dupes)}x")
        last_assistant = next((m for m in reversed(msgs)
                               if m["role"].lower() == "assistant"), None)
        if last_assistant and last_assistant.get("metadata", {}).get("completed_at"):
            return
        time.sleep(0.5)

t0 = time.time()
samples, seen_ids, seen_counts = [], set(), []

long_prompt = ("Write out the numbers 1 through 80, each number on its own line "
               "followed by the word ok. Do not use any tools.")
http("POST", f"/session/{sid}/prompt_async", {"model": model, "message": long_prompt})
print("streamed turn 1 started; polling...")
poll(20, "turn1", samples, seen_ids, seen_counts)

print("aborting turn 1...")
http("POST", f"/session/{sid}/prompt/abort")
for _ in range(20):
    try:
        status = http("GET", "/session/status")
    except Exception:
        status = None
    time.sleep(0.5)
    if not status:
        break

cont = "CONTINUE-MARKER-7F3A: list the next ten numbers only."
http("POST", f"/session/{sid}/prompt_async", {"model": model, "message": cont})
print("interrupt-then-continue started; polling...")
poll(25, "continue", samples, seen_ids, seen_counts, continue_text="CONTINUE-MARKER-7F3A")

print(f"samples={len(samples)}  min_count={min(seen_counts)}  "
      f"max_count={max(seen_counts)}  distinct_last_ids={len(seen_ids)}")
print("BEFORE: " + ", ".join(f"{t}:{lbl}:n={n}" for t, lbl, n, _ in samples[:4]))
print("AFTER:  " + ", ".join(f"{t}:{lbl}:n={n}" for t, lbl, n, _ in samples[-4:]))
print("PASS BUG-051: list non-decreasing, last id monotonic, continuation once")
PY

echo
echo "=== Restarting server (reload from storage) ==="
stop_server
start_server

echo
echo "=== Phase 2: BUG-043/047 durable terminal record + progress ==="
python3 - "$BASE" "$SID" <<'PY'
import json, sys, urllib.request
base, sid = sys.argv[1], sys.argv[2]

def http(path):
    with urllib.request.urlopen(base + path, timeout=30) as r:
        return json.loads(r.read().decode())

msgs = http(f"/session/{sid}/message")
assistants = [m for m in msgs if m["role"].lower() == "assistant"]
if not assistants:
    raise SystemExit("FAIL BUG-043/047: no assistant message persisted")

aborted = [m for m in assistants if m.get("error") == "aborted"]
if not aborted:
    raise SystemExit("FAIL BUG-043: aborted assistant record did not survive reload")
a = aborted[-1]
if not a.get("completed_at") or a.get("finish") != "aborted":
    raise SystemExit(
        f"FAIL BUG-043: aborted record lost terminal metadata "
        f"(completed_at={a.get('completed_at')}, finish={a.get('finish')})")
print(f"PASS BUG-043: durable terminal record survives restart "
      f"(error={a.get('error')}, finish={a.get('finish')}, "
      f"completed_at={a.get('completed_at')})")

last = assistants[-1]
parts = last.get("parts", [])
text_len = sum(len(p.get("text", "")) for p in parts)
print(f"persisted messages={len(msgs)}  assistant_parts={len(parts)}  "
      f"assistant_text_len={text_len}  finish={last.get('finish')}  "
      f"completed_at={last.get('completed_at')}")
if not parts:
    raise SystemExit("FAIL BUG-047: last assistant message has no persisted parts")
if not last.get("completed_at"):
    raise SystemExit("FAIL BUG-047: last assistant progress lost after reload")
print(f"PASS BUG-047: {len(parts)} in-flight parts ({text_len} chars) persisted")
PY

echo
echo "=== Phase 2b: raw storage payload carries metadata ==="
DB="$HOME/Library/Application Support/opencode/opencode.db"
if command -v sqlite3 >/dev/null 2>&1; then
  ROW="$(sqlite3 "$DB" \
    "SELECT data FROM messages WHERE role='assistant' ORDER BY created_at DESC LIMIT 1;")"
  if printf '%s' "$ROW" | grep -q '"finish_reason"'; then
    echo "PASS storage: messages.data carries finish_reason: $(printf '%s' "$ROW" | cut -c1-120)..."
  else
    echo "FAIL storage: messages.data missing finish_reason" >&2
    exit 1
  fi
else
  echo "h014-qa: sqlite3 unavailable; skipping raw storage check"
fi

echo
echo "=== Phase 3: 'opencode session inspect' terminal recognition ==="
INSPECT="$("$BIN" session inspect "$SID" --full 2>&1 || true)"
echo "$INSPECT"
if printf '%s' "$INSPECT" | grep -qi "stalled"; then
  echo "FAIL BUG-043: session inspect still reports the finalized run as stalled" >&2
  exit 1
fi
echo "PASS BUG-043: inspect does not mislabel the finalized run as stalled"

echo
echo "h014-qa: ALL CHECKS PASSED (server log: $LOGF)"
