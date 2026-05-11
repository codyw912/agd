# AGD Verify Adopted Patch Implementation Plan

## Goal

Make `agd verify <commit>` confirm that the human adoption commit's actual patch matches `AGD-Patch-SHA256`, not only that the recorded agent base/tip can reproduce the hash.

## Scope

- Add one integration test that creates a commit with valid-looking AGD trailers but different adopted content.
- Recompute the adoption commit patch from its first parent to the commit under verification.
- Report `mismatch` when either the recorded agent patch hash or the adopted commit patch hash differs from the trailer.

## Status

- [x] Add failing integration coverage.
- [x] Implement adopted patch verification.
- [x] Run focused and full verification.
