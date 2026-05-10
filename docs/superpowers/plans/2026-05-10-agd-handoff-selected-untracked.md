# AGD Handoff Selected Untracked Files

## Goal

Extend `agd handoff` so humans can explicitly include selected untracked files without making untracked handoff the default.

## Scope

- Add `--include-untracked <path>` as an explicit selection mechanism.
- Keep bare `agd handoff` refusing untracked human files.
- Copy only selected untracked files into the default agent workspace.
- Reject unsafe paths and paths that are not untracked files.
- Keep the agent workspace clean precondition and no auto-commit behavior.

## Test

- Prove a selected untracked file is copied into the workspace as an untracked file.
- Prove unselected untracked files are not copied.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
