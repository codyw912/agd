# AGD Init Dirty Check Plan

## Goal

Refuse `agd init` when the human checkout has uncommitted changes so new agent workspaces always start from committed human state.

## Scope

- Check the human checkout worktree before writing AGD metadata or creating the default workspace.
- Report a clear error when tracked or untracked changes are present.
- Preserve existing successful `agd init` and `agd --json init` behavior for clean repositories.

## Steps

- [x] Add failing integration coverage for dirty `agd init`.
- [x] Implement the clean-check in project initialization.
- [x] Run focused and full verification.
