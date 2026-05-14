# AGD bless validates agent branch before creating adoption branch

## Goal

Prevent `agd bless <branch>` from leaving behind a human adoption branch when the
selected agent branch cannot be fetched from the managed workspace.

## Behavior

- Missing or otherwise unfetchable agent branches fail before the human checkout
  creates or switches to an adoption branch.
- Successful branch adoption keeps the existing behavior and still creates the
  requested adoption branch from the target branch.

## Verification

- Add an integration regression test for `agd bless agent/missing`.
- Run the focused regression test, related bless tests, formatting, full tests,
  and lint.
