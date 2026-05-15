# AGD sync targets named workspaces

## Goal

Let `agd sync` update non-default managed workspaces without requiring users to
switch project configuration or recreate workspaces.

## Behavior

- `agd sync --workspace <id>` updates the selected workspace's target mirrors.
- `agd sync --workspace <id> --rebase <branch>` rebases the selected workspace's
  agent branch.
- When run inside an agent workspace, `agd sync` defaults to that current
  workspace if `--workspace` is omitted.
- Existing human-checkout behavior keeps targeting the default workspace.

## Verification

- Add integration coverage for named workspace sync and current-workspace
  defaulting.
- Keep existing sync and sync rebase tests passing.
- Run focused sync tests, formatting, full tests, and lint.
