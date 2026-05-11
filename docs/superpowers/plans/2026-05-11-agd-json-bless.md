# AGD JSON Bless Implementation Plan

## Goal

Add structured JSON output for successful `agd --json bless <branch>` while preserving the existing human output for clean bless adoption.

## Scope

- Add one integration test for the default squash bless JSON output.
- Return a serializable bless result from the adoption layer instead of printing there.
- Keep existing human messages for squash, preserve, and merge adoption in the CLI layer.

## Status

- [x] Add failing integration coverage.
- [x] Implement JSON output.
- [x] Run focused and full verification.
