# AGD JSON PR Output

## Goal

Add structured output for `agd --json pr [branch]` so automation can consume the created pull request URL without parsing text.

## Scope

- Keep existing `agd pr [branch]` text output unchanged.
- Return `branch` and `url` when `--json` is supplied.
- Preserve existing fetch, push, PR body, and `gh pr create` behavior.

## Test

- Assert `agd --json pr <branch>` pushes the branch, invokes `gh`, and returns the branch/URL.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
