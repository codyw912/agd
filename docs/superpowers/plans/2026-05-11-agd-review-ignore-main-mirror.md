# AGD Review Ignore Main Mirror Plan

## Goal

Keep `agd branches` focused on candidate agent branches when `main` exists as a synced mirror branch and the configured default target is a different branch.

## Scope

- Exclude `main` from review branch listings when it is a mirror branch.
- Preserve existing behavior for real agent branches.
- Keep this slice limited to branch listing; explicit `agd diff main` policy can be handled separately if needed.

## Steps

- [x] Add failing integration coverage for a `develop` default target with a synced `main` mirror.
- [x] Update review branch filtering.
- [x] Run focused and full verification.
