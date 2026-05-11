# AGD JSON Reset Workspace Output

## Goal

Add structured output for `agd --json reset-workspace` so automation can consume reset results without parsing text.

## Scope

- Keep existing `agd reset-workspace` text output unchanged.
- Return `workspace_id` and recreated workspace `path` when `--json` is supplied.
- Preserve existing clean-workspace checks, guardrail reinstall, metadata save, and operation locking.

## Test

- Assert `agd --json reset-workspace` recreates the default workspace and reports its id/path.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
