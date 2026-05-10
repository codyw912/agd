# AGD Handoff Tracked Diff

## Goal

Add the first `agd handoff` slice so a human can move messy tracked changes into the default agent workspace without committing them.

## Scope

- Discover the project from either checkout.
- Require the default agent workspace to be clean.
- Refuse untracked human files for now instead of silently omitting them.
- Export the human checkout diff from `HEAD` and apply it to the default workspace working tree.
- Record handoff metadata in the workspace git metadata.
- Do not commit automatically.

## Test

- Prove tracked human edits are applied to the default workspace as uncommitted changes.
- Prove a dirty default workspace is refused before applying handoff changes.

## Status

- [x] Failing tests added
- [x] Command implemented
- [x] Verification run
