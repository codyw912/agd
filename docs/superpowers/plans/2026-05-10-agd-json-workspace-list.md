# AGD JSON Workspace List

## Goal

Add structured output for `agd --json workspace list` so plugin consumers can read workspace records without parsing tabular text.

## Scope

- Keep existing `agd workspace list` text output unchanged.
- Return all project workspaces under a `workspaces` array when `--json` is supplied.
- Reuse the same workspace fields already used by JSON status output.

## Test

- Assert the default workspace is present with `id`, `status`, and `path`.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
