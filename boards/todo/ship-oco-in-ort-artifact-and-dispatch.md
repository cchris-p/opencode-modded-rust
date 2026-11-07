---
id: "INFRA-016"
title: "Ship oco with the ORT artifact and dispatch as argv[0]"
priority: "P1"
type: "feature"
area: "INFRA"
spec: "scripts/publish-cli-artifact.sh; crates/opencode-cli/src/main.rs; ~/standards/handoffs/H-014-ort-native-oco-handoff.md"
status: "todo"
created: "2026-11-07"
updated: "2026-11-07"
---

# Ship oco With The ORT Artifact And Dispatch As argv[0]

## Summary

Make `oco` a first-class entry point of the ORT distribution so it ships on every
artifact iteration and needs no separate repo or Node install.

## Scope

### ORT binary

- Detect the executable basename at startup. When it is `oco` (or
  `opencommit`), dispatch to the new `oco` command instead of expecting a
  subcommand.
- Keep `ort oco` working for explicit use and for scripts.
- No behavior change when invoked as `opencode-modded-rust`.

### Artifact packaging

- In `scripts/publish-cli-artifact.sh`, include an `oco` symlink in the staged
  tarball (or have the installer create it). Either way, the installed revision
  directory must expose both `opencode-modded-rust` and `oco`.
- Do not change the asset name, `.sha256`, or `.rev` contract.

### Standards installer

`~/standards/utilities/ort-install` currently copies only the binary. Update it
to also create `<rev-dir>/oco -> opencode-modded-rust` after extraction, and to
leave `bin/current/oco` resolvable.

### Verification

- `ort oco --help` and `opencode-modded-rust oco --help` match.
- A symlink/copy named `oco` runs the commit flow (argv[0] dispatch).
- `ort-install --dry-run` and a real install on a clean root produce
  `bin/<rev>/oco`.

## Out Of Scope

- The standards `~/.local/bin/oco` user link and npm uninstall (standards
  `STD-155`).
- Changing the release tag or repo.

## Acceptance Criteria

- [ ] `oco` resolves through `bin/current/oco` after `ort-install`.
- [ ] Invoking the binary as `oco` runs the `oco` command.
- [ ] `ort oco` still works.
- [ ] Packaging is reproducible and the checksum/rev files are unchanged in
      format.

## Related

- `CLI-014`, `CLI-015`, `CLI-016` — the implementation.
- Standards `H-014` / `STD-155` — user link, npm removal, docs.
