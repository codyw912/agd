# AGD cleanup defaults to current workspace

## Goal

Make cleanup commands consistent with other workspace-aware commands when run
from inside a named agent workspace.

## Behavior

- `agd discard <branch>` run inside a named workspace discards from that current
  workspace when `--workspace` is omitted.
- `agd reset-workspace` run inside a named workspace resets that current
  workspace when `--workspace` is omitted.
- Existing human-checkout behavior keeps targeting the default workspace.
- Explicit `--workspace <id>` behavior remains unchanged.

## Verification

- Add integration coverage for current-workspace defaulting in `discard` and
  `reset-workspace`.
- Keep existing cleanup tests passing.
- Run focused cleanup tests, formatting, full tests, and lint.
