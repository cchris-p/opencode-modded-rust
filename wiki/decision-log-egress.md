# Decision Log Egress, Retention, and Privacy Policy

## Purpose

Define what the System-1 decision layer records, where it may travel, how long it
is kept, and how it is deleted.

This is the policy of record for `decision-log-egress`. It exists so the decision
layer can produce a useful, replayable, ownable dataset without becoming a secret
leak, a private-data export, or an unmanaged second transcript store.

## Principles

1. **Local by default.** Decision records stay on the machine that produced them
   unless an explicit policy sends a specific class elsewhere.
2. **Minimize state.** Log the smallest representation that supports replay,
   evaluation, and audit. Prefer hashes, identifiers, spans, and structured
   features over raw prompt or file content.
3. **Sensitivity before engine.** The sensitivity tier of a decision class
   determines which engine may see it. A hosted engine is never the default for a
   sensitive class.
4. **Provenance is mandatory.** Every record names the decision class, engine,
   model, policy version, timestamp, confidence, cost, latency, and outcome.
5. **Append-only for runtime.** The runtime appends; it does not rewrite history.
   Corrections are appended as new records.
6. **Deletion is real.** Retention and deletion apply to the log, derived training
   sets, and cached copies. Deleting a record does not require deleting the task
   outcome it references, but the link must be removed.
7. **No secrets, ever.** API keys, tokens, credentials, cookies, private keys, and
   secret values are never logged, even in redacted form.
8. **No silent sync.** The decision log is not synchronized to the central session
   archive or any remote store unless the policy explicitly opts it in.

## What is logged

Each decision record is append-only JSON Lines. The exact schema is versioned; the
minimum fields are:

```json
{
  "schema": "decision-log/v1",
  "id": "dec_01J...",
  "ts": "2026-10-24T12:00:00Z",
  "session_id": "ses_...",
  "task_id": "task_...",
  "class": "review.triage",
  "policy_version": "policy-2026-10-24.3",
  "capability_profile": "deepseek-flash-2026-10",
  "engine": "jev-latest",
  "engine_kind": "hosted",
  "model": "deepseek/deepseek-flash",
  "sensitivity": "internal",
  "input_digest": "sha256:...",
  "input_summary": {
    "tokens": 812,
    "spans": 4,
    "redactions": 0
  },
  "question": {
    "type": "choice",
    "options": ["blocking", "non_blocking", "noise", "approve"]
  },
  "result": {
    "choice": "blocking",
    "confidence": 0.91,
    "rationale_code": "missing_test"
  },
  "cost": {
    "credits": 0.000021,
    "usd_estimate": 0.000021,
    "input_tokens": 812,
    "output_tokens": 0
  },
  "latency_ms": 84,
  "outcome": {
    "status": "confirmed",
    "source": "human",
    "detail": "review comment was blocking"
  },
  "egress": {
    "allowed": true,
    "reason": "internal class, hosted engine permitted",
    "redaction_profile": "default"
  }
}
```

Fields that are deliberately excluded or minimized:

- raw prompt text and full file contents,
- raw transcript text,
- secrets and credentials,
- personal data beyond what the class requires,
- any content the sensitivity policy marks as local-only.

## Sensitivity tiers

| Tier | Examples | Hosted engine | Local engine | Retention default |
| --- | --- | --- | --- | --- |
| `public` | open-source repo metadata, generic errors | allowed | allowed | 180 days |
| `internal` | task shapes, review classifications, board state | allowed with minimization | allowed | 90 days |
| `sensitive` | private research, customer data, non-public strategy | denied by default | allowed | 30 days |
| `secret` | keys, tokens, credentials, cookies, private keys | denied | denied (do not log) | never |

The tier is a property of the decision class and may be raised by the runtime when
the state contains a detected sensitive value. It may not be lowered without an
explicit policy change.

## Egress rules

### Hosted engines (Jev)

A hosted engine may receive a decision request only when all of the following are
true:

- the class is `public` or `internal`,
- the request passed the redaction profile,
- the state is minimized to the class's declared fields,
- the class is enabled for the active capability profile,
- the session has remaining decision budget,
- the policy version permits hosted egress.

The request must not include:

- secrets, credentials, or tokens,
- raw private transcripts,
- private research content,
- personal data not required by the class,
- absolute paths that identify a user or private project when a stable identifier
  suffices.

### Local engines (Laya)

A local engine may receive `sensitive` classes when the model and runtime are
local. `secret` content is never logged or sent to any engine; the runtime must
stop or redact before the decision boundary.

### No engine

Deterministic rules and stub engines are always allowed. They are the fallback when
egress is denied.

## Retention

Default schedule, configurable by policy:

- **Raw decision records:** 90 days for `public` and `internal`, 30 days for
  `sensitive`, never for `secret`.
- **Aggregates and metrics:** 1 year, with no raw content.
- **Training exports:** created only by an explicit fine-tune job, with its own
  manifest, retention date, and deletion command.
- **Replay fixtures:** held-out samples may be retained longer if they are
  scrubbed, labelled, and covered by the same sensitivity tier.

Deletion:

- a record can be deleted by id, session, class, or date range;
- derived training exports record which record ids they consumed;
- deleting a source record marks dependent exports for regeneration or removal;
- deletion is logged as a tombstone with the reason and scope, not the content.

## Access and storage

- Stored outside git, under the product's local data directory.
- File permissions restricted to the user.
- Not synchronized to the central session archive by default.
- Not uploaded to a backup or object store unless the policy explicitly opts in.
- Read access is local; aggregate reports may be shared without raw records.

## Replay

The log is replayable because it records:

- the decision class and input digest,
- the minimized input summary,
- the policy version,
- the capability profile,
- the engine and model,
- the result and confidence,
- the downstream outcome.

Replay may require the original workspace or a scrubbed fixture. If the original
state is unavailable, replay uses the recorded digest and summary to verify the
decision path, not to reconstruct private content.

## Fine-tune export

A fine-tune export:

- uses only records with an explicit `training_allowed: true` flag,
- excludes `sensitive` and `secret` tiers unless a local-only job is used,
- splits by task and time to avoid leakage,
- records the source record ids and the policy version,
- is deleted when its retention date passes or its source records are deleted.

## Change control

Changing this policy is a policy change, not an implementation detail. It must:

- be recorded in this document and in `invariants/decision-layer.md` if the
  authority or egress rule changes,
- be reviewed by a human,
- state the classes affected, the retention change, and the deletion plan,
- preserve the `secret` tier prohibition.

## Board

- `FUTURE-PROOFING-008` — define and enforce this policy.
- `DECISION-005` — implement the decision log and replay.
- `DECISION-014` — fine-tune exports and engine swap.
