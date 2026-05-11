# AGD Review Reject Main Mirror Plan

## Goal

Prevent explicit review commands from treating a synced `main` mirror branch as agent work when the project default target is another branch.

## Scope

- Reject `agd log main`, `agd diff main`, and `agd files main` as non-review branches.
- Reuse the same review-branch predicate used by `agd branches`.
- Preserve existing behavior for real agent branches.

## Steps

- [x] Add failing integration coverage for explicit review commands against a synced `main` mirror.
- [x] Update review branch resolution to reject non-review branches.
- [x] Run focused and full verification.
