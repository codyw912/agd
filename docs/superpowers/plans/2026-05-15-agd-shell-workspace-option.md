# AGD shell targets named workspaces

## Goal

Let `agd shell` enter a selected managed workspace instead of always entering
the default workspace.

## Behavior

- `agd shell --workspace <id>` starts the user's shell in the selected workspace.
- When run inside an agent workspace, `agd shell` defaults to that current
  workspace if `--workspace` is omitted.
- Existing human-checkout behavior keeps targeting the default workspace.
- `AGD_WORKSPACE_ID` reflects the selected workspace.

## Verification

- Add integration coverage for explicit named workspace shell and
  current-workspace defaulting.
- Keep the existing default shell test passing.
- Run focused shell tests, formatting, full tests, and lint.
