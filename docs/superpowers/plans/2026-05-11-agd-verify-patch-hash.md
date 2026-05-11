# AGD Verify Patch Hash Implementation Plan

## Goal

Add `AGD-Patch-SHA256` to squash adoption commits and teach `agd verify <commit>` to recompute and verify it.

## Scope

- Define the stable patch serialization for this slice as `git diff --binary --full-index --no-ext-diff <base> <agent-tip>`.
- Include the SHA-256 hex digest in squash adoption trailers, including `bless --continue`.
- Recompute that hash from AGD trailers in `agd verify` and report `verified` or `mismatch`.

## Status

- [x] Add failing integration coverage.
- [x] Implement patch hash trailers and verification.
- [x] Run focused and full verification.
