# AGD JSON Bless Abort Implementation Plan

## Goal

Add structured JSON output for `agd --json bless --abort` while preserving the existing human output for `agd bless --abort`.

## Scope

- Add one integration test that drives a conflicted squash bless through `--json bless --abort`.
- Return a serializable abort result from the adoption layer instead of printing there.
- Dispatch either JSON or human text in the CLI layer.

## Status

- [x] Add failing integration coverage.
- [x] Implement JSON output.
- [x] Run focused and full verification.
