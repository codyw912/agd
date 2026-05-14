# AGD review commands target named workspaces

## Goal

Let review commands inspect non-default managed workspaces without requiring users
to manually enter those workspace directories.

## Behavior

- `agd branches --workspace <id>` lists review branches from the selected workspace.
- `agd log --workspace <id> <branch>`, `agd diff --workspace <id> <branch>`,
  and `agd files --workspace <id> <branch>` read from the selected workspace.
- JSON output uses the same selected workspace.
- Existing default-workspace behavior remains unchanged when `--workspace` is not
  provided.

## Verification

- Add integration coverage for text and JSON review commands against a named
  workspace.
- Run focused review tests, formatting, full tests, and lint.
