# AGD Shell Command

## Goal

Add `agd shell` so a human can enter the default managed agent workspace through AGD instead of manually locating the clone.

## Scope

- Discover the current project from either the human checkout or an agent workspace.
- Launch the user's configured shell in the default workspace path.
- Set AGD environment metadata for child processes:
  - `AGD_WORKSPACE=1`
  - `AGD_PROJECT_ID=<project id>`
  - `AGD_WORKSPACE_ID=<workspace id>`
- Propagate a non-zero shell exit as an AGD command failure.

## Test

- Add an integration test with a fake shell executable.
- The fake shell records its current directory and AGD environment.
- The test asserts that `agd shell` starts in the default workspace with expected metadata.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
