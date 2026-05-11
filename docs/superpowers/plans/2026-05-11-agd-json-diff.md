# AGD JSON Diff Implementation Plan

## Goal

Add structured JSON output for `agd --json diff <branch>` while preserving the existing raw human `agd diff` output.

## Scope

- Add one integration test for JSON review diff output.
- Add a review-layer result type containing the resolved branch and patch text.
- Route `--json diff` through `json_output::print`.

## Status

- [x] Add failing integration coverage.
- [x] Implement JSON output.
- [x] Run focused and full verification.
