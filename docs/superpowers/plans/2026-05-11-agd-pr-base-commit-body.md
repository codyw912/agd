# AGD PR Base Commit Body Plan

## Goal

Include the exact base commit in generated `agd pr` bodies so PR reviewers get stable provenance context instead of only a base branch name.

## Scope

- Add `Base commit: <sha>` to the existing PR body summary.
- Preserve current push, `gh pr create`, text output, and JSON output behavior.
- Keep this slice limited to PR body content.

## Steps

- [x] Add failing integration coverage for base commit in the `gh pr create` body argument.
- [x] Add base commit lookup to PR body generation.
- [x] Run focused and full verification.
