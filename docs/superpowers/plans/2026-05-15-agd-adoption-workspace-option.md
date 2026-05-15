# AGD adoption targets named workspaces

## Goal

Let human-checkout adoption commands use agent branches from a selected managed
workspace instead of only the default workspace.

## Behavior

- `agd bless --workspace <id> <branch>` adopts the branch from the selected
  workspace.
- `agd pr --workspace <id> <branch>` publishes an agent branch from the selected
  workspace.
- `agd pr --bless --workspace <id> <branch>` adopts from the selected workspace
  and opens the human-owned PR.
- When run inside an agent workspace, existing current-workspace defaults remain
  available.

## Verification

- Add integration coverage for named workspace bless and PR bless.
- Keep existing bless and PR behavior passing.
- Run focused tests, formatting, full tests, and lint.
