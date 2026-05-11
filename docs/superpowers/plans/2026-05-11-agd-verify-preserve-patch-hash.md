# AGD Verify Preserve Patch Hash Implementation Plan

## Goal

Add `AGD-Patch-SHA256` to preserve-mode adoption commits and make `agd verify <commit>` verify each replayed commit against its `AGD-Agent-Commit`.

## Scope

- Add integration coverage for verifying preserve-mode adoption commits.
- Compute each preserve patch hash from the original agent commit's first-parent patch.
- When `AGD-Agent-Commit` is present, verify against that commit patch instead of the whole branch base/tip patch.

## Status

- [x] Add failing integration coverage.
- [x] Implement preserve patch hash trailers and verification.
- [x] Run focused and full verification.
