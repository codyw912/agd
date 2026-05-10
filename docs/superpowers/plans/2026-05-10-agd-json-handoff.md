# AGD JSON Handoff Output

## Goal

Add structured output for `agd --json handoff` so automation can consume handoff results without parsing text.

## Scope

- Keep existing `agd handoff` text output unchanged.
- Return `workspace_id`, `human_head`, tracked changed files, and selected untracked files when `--json` is supplied.
- Preserve existing clean-workspace, untracked-file, copy, metadata, and no-auto-commit behavior.

## Test

- Assert `agd --json handoff --include-untracked <path>` applies tracked changes, copies selected untracked files, and reports both file lists.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
