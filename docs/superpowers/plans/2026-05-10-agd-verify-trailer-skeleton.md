# AGD Verify Trailer Skeleton

## Goal

Add a conservative `agd verify <commit>` command that can inspect AGD provenance trailers without claiming cryptographic patch verification before `AGD-Patch-SHA256` serialization is defined.

## Scope

- Discover the AGD project from either checkout.
- Read commit trailers from the requested commit.
- Report missing AGD provenance metadata.
- Report missing `AGD-Patch-SHA256` metadata for AGD adoption commits.
- Exit nonzero until a future slice can recompute and compare a stable patch hash.

## Test

- Verify a non-AGD commit reports missing AGD metadata.
- Verify a current AGD adoption commit reports missing patch hash metadata.

## Status

- [x] Failing tests added
- [x] Command implemented
- [x] Verification run
