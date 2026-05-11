# AGD JSON Bless Continue Implementation Plan

## Goal

Add structured JSON output for `agd --json bless --continue` while preserving the existing human output for `agd bless --continue`.

## Scope

- Add one integration test that resolves a conflicted squash bless through `--json bless --continue`.
- Return a serializable continue result from the adoption layer instead of printing there.
- Dispatch either JSON or human text in the CLI layer.

## Status

- [x] Add failing integration coverage.
- [x] Implement JSON output.
- [x] Run focused and full verification.
