#!/usr/bin/env python3
"""Bulk-rename ORT sessions on this machine with LLM-generated titles.

CLI-018: read every session from the ORT SQLite database, summarize each
session's user inputs with the configured ORT model, and write a short title
back to the database. Defaults to a dry run; pass --apply to write.

The script is intentionally standalone (stdlib only) so it can be removed after
the one-off rename pass.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any


TITLE_PROMPT = """You name AI coding sessions. For each session below, write a concise 3-8 word title describing what the user was working on. Prefer board/task IDs (for example FEAT-080, BUG-075) when present. Use the same language as the user. No quotes, no trailing punctuation, no generic titles like "New session" or "Untitled".
Return ONLY one JSON object mapping each session id to its title.

SESSIONS:
"""


def connect_ro(path: Path) -> sqlite3.Connection:
    return sqlite3.connect(f"{path.resolve().as_uri()}?mode=ro", uri=True)


def data_dirs() -> list[Path]:
    candidates: list[Path] = []
    for key in ("OPENCODE_DATA_DIR", "OPENCODE_ORT_DATA_DIR"):
        value = os.environ.get(key)
        if value:
            candidates.append(Path(value).expanduser())
    home = Path.home()
    if sys.platform == "darwin":
        support = home / "Library" / "Application Support"
        candidates.extend([support / "opencode-ort", support / "opencode"])
    xdg = os.environ.get("XDG_DATA_HOME")
    base = Path(xdg).expanduser() if xdg else home / ".local" / "share"
    candidates.extend([base / "opencode-ort", base / "opencode"])

    unique: list[Path] = []
    seen: set[str] = set()
    for candidate in candidates:
        key = str(candidate)
        if key not in seen:
            seen.add(key)
            unique.append(candidate)
    return unique


def candidate_db_paths() -> list[Path]:
    return [directory / "opencode.db" for directory in data_dirs()]


def database_has_sessions(path: Path) -> bool:
    try:
        con = connect_ro(path)
        try:
            row = con.execute(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'sessions'"
            ).fetchone()
            return row is not None
        finally:
            con.close()
    except sqlite3.Error:
        return False


def default_db_path() -> Path:
    candidates = candidate_db_paths()
    for candidate in candidates:
        if candidate.is_file() and database_has_sessions(candidate):
            return candidate
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return candidates[0]


def default_cli_path() -> str:
    candidates = [
        os.environ.get("OPENCODE_RUST_BIN"),
        shutil.which("opencode-modded-rust"),
        str(Path.home() / ".local" / "share" / "ort" / "bin" / "current" / "opencode-modded-rust"),
        str(Path(__file__).resolve().parent.parent / "target" / "debug" / "opencode-modded-rust"),
    ]
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return candidate
    return "opencode-modded-rust"


def config_env_path() -> Path:
    config_dir = os.environ.get("OPENCODE_CONFIG_DIR")
    base = Path(config_dir).expanduser() if config_dir else Path.home() / ".config" / "opencode"
    return base / ".env"


def load_shared_env(target: dict[str, str]) -> None:
    try:
        lines = config_env_path().read_text().splitlines()
    except OSError:
        return
    for line in lines:
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip()
        if not key or not all(char.isalnum() or char == "_" for char in key):
            continue
        if value and key not in target:
            target[key] = value


def auth_paths() -> list[Path]:
    return [directory / "auth.json" for directory in data_dirs()]


def load_auth_providers() -> set[str]:
    providers: set[str] = set()
    for path in auth_paths():
        try:
            data = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError):
            continue
        if isinstance(data, dict):
            providers.update(str(key) for key in data)
    return providers


def seed_auth(data_dir: Path) -> None:
    merged: dict[str, Any] = {}
    for path in reversed(auth_paths()):
        try:
            data = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError):
            continue
        if isinstance(data, dict):
            merged.update(data)
    if not merged:
        return
    destination = data_dir / "auth.json"
    destination.write_text(json.dumps(merged))
    try:
        destination.chmod(0o600)
    except OSError:
        pass


def fallback_models(env: dict[str, str], providers: set[str]) -> list[str]:
    models: list[str] = []
    if env.get("OPENCODE_API_KEY") or "opencode-go" in providers:
        models.append("opencode-go/deepseek-v4.1-flash")
    if env.get("DEEPSEEK_API_KEY") or "deepseek" in providers:
        models.append("deepseek/deepseek-flash")
    if env.get("ANTHROPIC_API_KEY") or "anthropic" in providers:
        models.append("anthropic/claude-haiku-4-5")
    if env.get("OPENAI_API_KEY") or "openai" in providers:
        models.append("openai/gpt-4o-mini")
    if env.get("GEMINI_API_KEY") or env.get("GOOGLE_API_KEY") or "google" in providers:
        models.append("google/gemini-2.5-flash")
    if env.get("NOVITA_API_KEY") or "novita-ai" in providers:
        models.append("novita-ai/deepseek/deepseek-v4-pro-0813")
    return models


def running_ort_processes() -> list[str]:
    try:
        output = subprocess.run(
            ["ps", "-eo", "pid=,args="],
            capture_output=True,
            text=True,
            check=False,
        ).stdout
    except OSError:
        return []
    matches: list[str] = []
    for line in output.splitlines():
        line = line.strip()
        if "opencode-modded-rust" not in line:
            continue
        if " rename-ort-sessions" in line:
            continue
        if re.search(r"opencode-modded-rust\s+(serve|tui)\b", line):
            matches.append(line)
    return matches


def extract_text_parts(data: str) -> list[str]:
    try:
        payload = json.loads(data or "{}")
    except json.JSONDecodeError:
        return []
    texts: list[str] = []
    for part in payload.get("parts", []):
        part_type = part.get("part_type")
        text = None
        if isinstance(part_type, dict) and part_type.get("type") == "text":
            text = part_type.get("text")
        elif isinstance(part_type, str) and part_type == "text":
            text = part.get("text")
        if isinstance(text, str) and text.strip():
            texts.append(text.strip())
    return texts


def compact(text: str, max_chars: int) -> str:
    return re.sub(r"\s+", " ", text).strip()[:max_chars]


def load_sessions(db: Path, workspace: str | None, max_messages: int, max_chars: int) -> list[dict[str, Any]]:
    con = connect_ro(db)
    try:
        sql = "SELECT id, title, directory, created_at, updated_at FROM sessions"
        params: list[Any] = []
        if workspace:
            sql += " WHERE directory = ?"
            params.append(workspace)
        sql += " ORDER BY created_at"
        sessions = [
            {
                "id": row[0],
                "old_title": row[1],
                "directory": row[2],
                "created_at": row[3],
                "updated_at": row[4],
                "messages": [],
            }
            for row in con.execute(sql, params).fetchall()
        ]
        by_id = {session["id"]: session for session in sessions}
        for session_id, data in con.execute(
            "SELECT session_id, data FROM messages WHERE role = 'user' ORDER BY session_id, created_at"
        ):
            session = by_id.get(session_id)
            if session is None or len(session["messages"]) >= max_messages:
                continue
            for text in extract_text_parts(data):
                if len(session["messages"]) >= max_messages:
                    break
                session["messages"].append(compact(text, max_chars))
        return sessions
    finally:
        con.close()


def build_prompt(sessions: list[dict[str, Any]]) -> str:
    prompt = TITLE_PROMPT
    for session in sessions:
        prompt += f"\n[{session['id']}]\n"
        if session["messages"]:
            for message in session["messages"]:
                prompt += f"- {message}\n"
        else:
            prompt += "- (no user messages)\n"
    return prompt


def assistant_text(stdout: str) -> str:
    marker = "Assistant:"
    index = stdout.rfind(marker)
    return stdout[index + len(marker) :].strip() if index >= 0 else stdout.strip()


def parse_titles(text: str) -> dict[str, str]:
    start = text.find("{")
    end = text.rfind("}")
    if start < 0 or end < start:
        return {}
    try:
        payload = json.loads(text[start : end + 1])
    except json.JSONDecodeError:
        return {}
    if not isinstance(payload, dict):
        return {}
    return {str(key): str(value) for key, value in payload.items() if isinstance(value, str)}


def clean_title(title: str) -> str | None:
    title = title.splitlines()[0].strip()
    title = title.strip(" \t\"'`*")
    title = re.sub(r"\s+", " ", title)
    if not title:
        return None
    if title.lower() in {"new session", "session", "untitled", "untitled session"}:
        return None
    return title[:100]


def fallback_title(session: dict[str, Any]) -> str:
    if session["messages"]:
        words = session["messages"][0].split()
        return " ".join(words[:8])[:100]
    created = dt.datetime.fromtimestamp(session["created_at"] / 1000).strftime("%Y-%m-%d")
    return f"Empty session {created}"


def call_llm(cli: str, model: str | None, prompt: str, timeout: int) -> str:
    with tempfile.TemporaryDirectory(prefix="ort-title-data-") as data_dir, tempfile.TemporaryDirectory(
        prefix="ort-title-work-"
    ) as work_dir:
        env = os.environ.copy()
        load_shared_env(env)
        env["OPENCODE_DATA_DIR"] = data_dir
        env.setdefault("OPENCODE_SERVER_LOG", "0")
        seed_auth(Path(data_dir))
        command = [cli, "run", "--dir", work_dir, "--format", "default", "--agent", "build"]
        if model:
            command.extend(["--model", model])
        command.append(prompt)
        result = subprocess.run(
            command,
            env=env,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
        if result.returncode != 0:
            stderr = compact(result.stderr, 500)
            raise RuntimeError(f"{cli} run exited {result.returncode}: {stderr}")
        return assistant_text(result.stdout)


def generate_titles(
    sessions: list[dict[str, Any]],
    cli: str,
    models: list[str],
    batch_size: int,
    timeout: int,
) -> dict[str, str]:
    titles: dict[str, str] = {}
    for start in range(0, len(sessions), batch_size):
        batch = sessions[start : start + batch_size]
        parsed: dict[str, str] = {}
        last_error: Exception | None = None
        for model in models:
            label = model or "product default"
            try:
                response = call_llm(cli, model, build_prompt(batch), timeout)
                parsed = parse_titles(response)
                if parsed:
                    break
                last_error = RuntimeError(
                    f"{label} returned no parseable titles: {compact(response, 200)!r}"
                )
            except Exception as error:
                last_error = error
                print(
                    f"rename-ort-sessions: {label} failed: {compact(str(error), 200)}",
                    file=sys.stderr,
                )
        if not parsed:
            raise RuntimeError(f"all title models failed: {last_error}")
        for session in batch:
            raw = parsed.get(session["id"])
            title = clean_title(raw) if raw else None
            titles[session["id"]] = title or fallback_title(session)
    return titles


def load_proposals(path: Path) -> dict[str, str]:
    payload = json.loads(path.read_text())
    if not isinstance(payload, dict):
        raise RuntimeError("proposals file must be a JSON object")
    return {str(key): str(value) for key, value in payload.items()}


def print_plan(sessions: list[dict[str, Any]], titles: dict[str, str]) -> None:
    width = max((len(session["id"]) for session in sessions), default=12)
    for session in sessions:
        new_title = titles.get(session["id"], "")
        print(f"{session['id']:<{width}}  {new_title}")
        print(f"{'':<{width}}  old: {compact(session['old_title'], 90)}")


def apply_titles(
    db: Path,
    sessions: list[dict[str, Any]],
    titles: dict[str, str],
    touch: bool,
    backup: bool,
) -> Path | None:
    backup_path: Path | None = None
    if backup:
        stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
        backup_path = db.with_name(f"{db.name}.rename-backup-{stamp}")
        source = connect_ro(db)
        target = sqlite3.connect(backup_path)
        try:
            source.backup(target)
        finally:
            target.close()
            source.close()

    now_ms = int(time.time() * 1000)
    con = sqlite3.connect(db)
    try:
        con.execute("PRAGMA busy_timeout = 5000")
        con.execute("BEGIN IMMEDIATE")
        for session in sessions:
            title = titles.get(session["id"])
            if not title:
                continue
            updated_at = now_ms if touch else session["updated_at"]
            con.execute(
                "UPDATE sessions SET title = ?, updated_at = ? WHERE id = ?",
                (title, updated_at, session["id"]),
            )
        con.commit()
    except Exception:
        con.rollback()
        raise
    finally:
        con.close()
    return backup_path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Bulk-rename ORT sessions with titles generated by the configured ORT model."
    )
    parser.add_argument("--db", type=Path, default=None, help="ORT SQLite database (default: auto-detect)")
    parser.add_argument(
        "--workspace",
        default=None,
        help="Only rename sessions for this directory (default: all sessions on this machine)",
    )
    parser.add_argument("--cli", default=None, help="opencode-modded-rust binary to call for title generation")
    parser.add_argument("--model", default=None, help="Model override for title generation")
    parser.add_argument("--max-messages", type=int, default=3, help="User messages sampled per session")
    parser.add_argument("--max-chars", type=int, default=400, help="Characters per sampled message")
    parser.add_argument("--batch-size", type=int, default=20, help="Sessions per model call")
    parser.add_argument("--timeout", type=int, default=240, help="Seconds per model call")
    parser.add_argument("--apply", action="store_true", help="Write titles to the database")
    parser.add_argument("--force", action="store_true", help="Apply while ORT servers are running")
    parser.add_argument("--touch", action="store_true", help="Bump updated_at when applying")
    parser.add_argument("--no-backup", action="store_true", help="Skip the pre-apply database backup")
    parser.add_argument("--proposals-out", type=Path, default=None, help="Write generated titles as JSON")
    parser.add_argument("--proposals-in", type=Path, default=None, help="Apply titles from a JSON file")
    args = parser.parse_args()

    db = Path(args.db).expanduser() if args.db else default_db_path()
    if not db.is_file():
        print(f"rename-ort-sessions: database not found: {db}", file=sys.stderr)
        print("candidates checked:", file=sys.stderr)
        for candidate in candidate_db_paths():
            print(f"  {candidate}", file=sys.stderr)
        return 2
    if not database_has_sessions(db):
        print(
            f"rename-ort-sessions: {db} has no 'sessions' table; this is not an ORT database",
            file=sys.stderr,
        )
        print("candidates checked:", file=sys.stderr)
        for candidate in candidate_db_paths():
            state = "ok" if candidate.is_file() and database_has_sessions(candidate) else "no"
            print(f"  {state}  {candidate}", file=sys.stderr)
        print("Pass --db <path> to select the ORT database explicitly.", file=sys.stderr)
        return 2

    workspace = str(Path(args.workspace).resolve()) if args.workspace else None
    sessions = load_sessions(db, workspace, args.max_messages, args.max_chars)
    if not sessions:
        print("rename-ort-sessions: no sessions matched", file=sys.stderr)
        return 1

    processes = running_ort_processes()
    if processes and args.apply and not args.force:
        print("rename-ort-sessions: refusing to apply while ORT is running:", file=sys.stderr)
        for process in processes:
            print(f"  {process}", file=sys.stderr)
        print("Quit ORT for this machine, or re-run with --force (touches updated_at).", file=sys.stderr)
        return 2

    if args.proposals_in:
        titles = load_proposals(args.proposals_in)
    else:
        cli = args.cli or default_cli_path()
        if args.model:
            models = [args.model]
        else:
            env = os.environ.copy()
            load_shared_env(env)
            models = fallback_models(env, load_auth_providers())
            if not models:
                models = [None]
        titles = generate_titles(sessions, cli, models, args.batch_size, args.timeout)

    for session in sessions:
        titles.setdefault(session["id"], fallback_title(session))

    print(f"database: {db}")
    print(f"sessions: {len(sessions)}")
    print_plan(sessions, titles)

    if args.proposals_out:
        args.proposals_out.write_text(json.dumps(titles, indent=2, sort_keys=True) + "\n")

    if not args.apply:
        print("\ndry run: pass --apply to write these titles")
        return 0

    touch = args.touch or bool(processes)
    backup_path = apply_titles(db, sessions, titles, touch=touch, backup=not args.no_backup)
    if backup_path:
        print(f"\nbackup: {backup_path}")
    print(f"applied {len(sessions)} titles")
    if processes:
        print("ORT is still running; restart it to see the new titles.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
    except Exception as error:
        print(f"rename-ort-sessions: {error}", file=sys.stderr)
        raise SystemExit(1)
