# AGD PR Reject Main Mirror Plan

## Goal

Prevent `agd pr main` from treating a synced `main` mirror branch as agent work when the configured default target is another branch.

## Scope

- Reject empty branch, default target, and `main` mirror selections in `agd pr`.
- Preserve existing PR behavior for real agent branches.
- Keep this slice limited to PR branch selection.

## Steps

- [x] Add failing integration coverage for `agd pr main` with a `develop` default target.
- [x] Update PR branch resolution to reject non-agent mirror branches.
- [x] Run focused and full verification.
