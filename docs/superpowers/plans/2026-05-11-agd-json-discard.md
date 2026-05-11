# AGD JSON Discard Output

## Goal

Add structured output for `agd --json discard <branch>` so automation can consume discard results without parsing text.

## Scope

- Keep existing `agd discard <branch>` text output unchanged.
- Return discarded `branch` and `workspace_id` when `--json` is supplied.
- Preserve existing clean-workspace checks and operation locking.

## Test

- Assert `agd --json discard <branch>` deletes the branch and reports `branch` and `workspace_id`.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
