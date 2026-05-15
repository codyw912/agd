# AGD status targets named workspaces

## Goal

Let `agd status` report a selected managed workspace instead of always reporting
the default workspace from the human checkout.

## Behavior

- `agd status --workspace <id>` reports the selected workspace path and pending
  branches.
- `agd status --json --workspace <id>` reports the selected workspace in the
  `workspace` object and uses that workspace for `agent_branches`.
- When run inside an agent workspace, `agd status` defaults to that current
  workspace if `--workspace` is omitted.
- Existing human-checkout behavior keeps targeting the default workspace.

## Verification

- Add integration coverage for explicit named workspace status and JSON status.
- Keep existing status behavior passing.
- Run focused status tests, formatting, full tests, and lint.
