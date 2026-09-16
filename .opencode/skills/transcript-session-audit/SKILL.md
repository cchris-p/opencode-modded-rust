---
name: transcript-session-audit
description: Transcript file audit, session review, exported chat markdown, "anything to note", tool-call quality, and OpenCode session correctness. Use ONLY when the user provides or points to a transcript file path, including a path in another repo, and asks to verify whether the session functioned correctly or identify process improvements in tool use, context gathering, reasoning, board/git handling, or handoff quality.
license: MIT
compatibility: opencode
metadata:
  audience: maintainers
  workflow: transcript-audit
---

# Transcript Session Audit

## Execution Gate

This skill cannot be executed unless the user provides a concrete transcript file location.

Before doing any transcript audit work, confirm the request includes a transcript file path. If it does not, do not inspect session history, infer a likely file, or audit the current conversation. Ask for the transcript file location and stop.

Acceptable inputs:

- An absolute path to a transcript markdown file
- A path relative to the current workspace
- A path in another local repository
- An explicitly attached or referenced transcript file that resolves to a readable local path

If the user asks for a session audit without providing a transcript file path, ask one short question for the transcript location and stop until they provide it.

Do not infer the target transcript from recent files, session memory, or the current conversation unless the user explicitly identifies that file.

## What it does

- Reviews an OpenCode transcript or exported session markdown as evidence
- Verifies whether the session completed the user's request correctly
- Identifies incorrect assumptions, missed context, unsafe actions, or incomplete verification
- Evaluates tool-call quality, including search/read/edit choices, shell usage, parallelization, and avoidance of destructive commands
- Checks workflow hygiene for boards, git, commits, PRs, QA handoff, and untracked files when those appear in the transcript
- Produces concrete improvement notes that can guide future sessions

## When to use it

Use this when the user provides a transcript path and asks questions like:

- "anything to note from this session?"
- "review this transcript"
- "did this session work correctly?"
- "what should the agent have done differently?"
- "audit the tool calls/context handling"
- "verify this session before I trust the outcome"

Use this for transcript/session analysis, not for implementing the original task described inside the transcript unless the user separately asks for follow-up work.

Do not use this as a replacement for live debugging when the user is reporting current product behavior and no transcript/session evidence is supplied.

## Evidence Sources

Start with the transcript file path the user provided. If the transcript points at repository state that can still be checked, corroborate important claims with current evidence.

Useful corroboration sources:

- Current `git status --short --branch`
- Recent `git log --oneline`
- `git branch --contains <commit>` for commits mentioned in the transcript
- Board lane directories or `bd` output when board items were moved
- Files named in the transcript when the outcome depends on their current content
- Local OpenCode session database only when the transcript is incomplete and the user wants deeper session forensics

Treat current state as corroboration, not as a reason to ignore what happened in the transcript.

## Workflow

1. Resolve and read the transcript file.

- Confirm the provided path exists and is readable.
- If the path is ambiguous or missing, ask for a precise path.
- If the transcript lives outside the current repo, read it by absolute path or the user-provided relative path without copying it into this repo.
- If the file is very large, read enough context to cover the original ask, major tool calls, edits, verification, final answer, and any referenced errors.

2. Identify the session goal.

- Extract the user's original ask from the transcript.
- Note any explicit constraints, such as commit requested, no PR requested, plan-only mode, board workflow, or dirty-worktree handling.
- If the transcript is only partial and the missing portion could change the conclusion, say that before making firm claims.

3. Reconstruct the actual path taken.

- List the meaningful phases: context gathering, decision points, edits, verification, commit/PR/board actions, final report.
- Distinguish observed actions from the assistant's internal intent or speculation.
- Track branch, worktree, board lane, and untracked-file state when mentioned.

4. Verify correctness against the ask.

- Did the session do what the user asked?
- Did it modify only intended files or state?
- Did it leave unrelated or concurrent work untouched?
- Did it ask for clarification when ambiguity carried real risk?
- Did it verify the result using an appropriate command or file read?
- Did the final answer disclose important residual state, such as untracked files or commits on a non-target branch?

5. Audit tool-call quality.

- Prefer `Read`, `Glob`, and `Grep` for file inspection over shell commands that duplicate those tools.
- Prefer `apply_patch` for manual edits.
- Prefer board-native commands such as `bd` for board lane moves.
- Use parallel tool calls when independent reads/searches can run together.
- Avoid noisy shell output, broad searches in generated directories, and shell pipelines that hide relevant output.
- Check whether commands were scoped to the intended working directory.
- Flag any destructive, risky, or overly broad command, even if it happened not to cause damage.

6. Audit context and reasoning quality.

- Did the agent inspect repository instructions before acting when needed?
- Did it use project-specific lane order, branch workflow, and config conventions rather than assumptions?
- Did it notice and handle concurrent work or unexpected files?
- Did it avoid overfitting to a word or command when the project had a more precise convention?
- Did it stop to ask the user only where ambiguity materially affected outcome?

7. Audit workflow hygiene.

- For git work, confirm branch, staged files, commit message, and included files were appropriate.
- For board work, confirm the lane move matched `board.toml` order and item IDs.
- For code changes, confirm tests or checks were run when needed.
- For docs/session audits, confirm the session did not accidentally commit generated artifacts.
- For PR workflows, confirm no merge happened without explicit user approval.

8. Report findings.

Lead with concise findings, ordered by importance.

Use this shape:

- `Issue:` what went wrong or could be improved
- `Evidence:` transcript line, command, file path, branch, or commit where possible
- `Impact:` why it matters
- `Better next time:` a concrete behavioral improvement

If nothing material is wrong, state that explicitly and still mention residual risks or unverified assumptions.

## Finding Categories

Use these categories when helpful:

- Correctness: outcome does not match the ask or only partially matches
- Safety: unrelated work, destructive commands, broad staging, or risky environment changes
- Context: missing project instructions, wrong branch assumptions, stale board interpretation
- Tooling: poor tool choice, avoidable noisy shell usage, missed parallelization, missing verification
- Communication: final answer omitted important state or overclaimed certainty
- Follow-up: a concrete action remains, such as cherry-picking a commit or removing an untracked artifact

## Severity Guidance

- High: likely wrong outcome, data loss risk, committed unrelated work, destructive command, or branch/PR workflow violation
- Medium: outcome probably usable but important state was omitted, verification was insufficient, or a follow-up is required
- Low: process inefficiency, noisy search, missed parallelization, or communication polish

Do not inflate minor style issues into findings. Prefer a short list of real, actionable observations.

## Output Requirements

- Keep the answer grounded in transcript evidence.
- Avoid re-litigating every tool call; focus on steps that changed risk or outcome.
- Mention any current-state checks you performed separately from transcript observations.
- If the transcript contains concurrent-work evidence, clearly separate what the audited session did from what another session or user likely did.
- If a commit is mentioned, verify whether it is on the current branch when that matters.
- If the user asked "anything to note", keep the final response concise and practical.

## Verification Checklist

Before finishing, confirm:

- A transcript file path was provided and read.
- The original ask and final session outcome were compared.
- Any claimed commit/branch/board state was either sourced from the transcript or checked live.
- Any unexpected/untracked files were called out if relevant.
- Process-improvement notes are specific enough to change future behavior.
- Uncertainty is explicit where the transcript is incomplete.
