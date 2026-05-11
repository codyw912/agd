# AGD Doctor Bless State Implementation Plan

## Goal

Make `agd doctor` detect incomplete AGD bless recovery state persisted at `.git/agd/bless.json`.

## Scope

- Add an integration test that creates a conflicted squash bless and runs `agd doctor`.
- Add a doctor check that fails when the human checkout has persisted bless state.
- Include recovery guidance to run `agd bless --continue` or `agd bless --abort`.

## Status

- [x] Add failing integration coverage.
- [x] Implement doctor bless state check.
- [x] Run focused and full verification.
