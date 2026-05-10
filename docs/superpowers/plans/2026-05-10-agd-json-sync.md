# AGD JSON Sync Output

## Goal

Add structured output for `agd --json sync` so automation can observe which target was synchronized and what ref changed.

## Scope

- Keep existing `agd sync` text output unchanged.
- Return `target`, `workspace_id`, `before`, and `after` when `--json` is supplied.
- Preserve all existing dirty/diverged checks and operation locking.

## Test

- Assert `agd --json sync` fast-forwards the default target and reports the updated ref.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
