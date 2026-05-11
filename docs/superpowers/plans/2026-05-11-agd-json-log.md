# AGD JSON Log Implementation Plan

## Goal

Add structured JSON output for `agd --json log <branch>` while preserving the existing human `agd log` output.

## Scope

- Add one integration test for JSON review log output.
- Add a review-layer result type for commit log data.
- Route `--json log` through `json_output::print`.

## Status

- [x] Add failing integration coverage.
- [x] Implement JSON output.
- [x] Run focused and full verification.
