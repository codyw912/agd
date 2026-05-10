# AGD JSON Workspace Create

## Goal

Add structured output for `agd --json workspace create <name>` so automation can create a workspace and consume its metadata without parsing text.

## Scope

- Keep existing `agd workspace create <name>` text output unchanged.
- Return the created workspace under a `workspace` object when `--json` is supplied.
- Reuse the same workspace fields used by JSON status and workspace list output.

## Test

- Assert the created workspace JSON includes `id`, `status`, and `path`.
- Assert the workspace is persisted and usable via `agd path --workspace`.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
