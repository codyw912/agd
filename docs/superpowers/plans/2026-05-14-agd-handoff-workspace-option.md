# AGD handoff targets named workspaces

## Goal

Let `agd handoff` move human checkout changes into a selected managed workspace
instead of always using the default workspace.

## Behavior

- `agd handoff --workspace <id>` applies tracked and selected untracked human
  changes to the selected workspace.
- When run inside an agent workspace, `agd handoff` defaults to that current
  workspace if `--workspace` is omitted.
- Existing human-checkout behavior keeps targeting the default workspace.
- Handoff metadata and JSON output report the selected workspace id.

## Verification

- Add integration coverage for explicit named workspace handoff and
  current-workspace defaulting.
- Keep existing handoff tests passing.
- Run focused handoff tests, formatting, full tests, and lint.
