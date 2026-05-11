# AGD Sync Main Mirror Plan

## Goal

Make `agd sync` update both the configured default target and `main` when `main` exists, matching the PRD default mirror branch contract.

## Scope

- Keep existing single-target sync output compatible by retaining the top-level `target`, `before`, and `after` fields.
- Add per-branch sync update details for multi-branch syncs.
- Update only mirror branches; keep `agent/*` branches untouched.

## Steps

- [x] Add failing integration coverage for syncing `main` plus a non-main default target.
- [x] Implement multi-target mirror sync.
- [x] Run focused and full verification.
