# AGD bless abort without pending state

## Goal

Make `agd bless --abort` report a clear error when there is no persisted AGD bless
operation to abort.

## Behavior

- If `.git/agd/bless.json` is absent, `agd bless --abort` fails with
  `no pending bless operation`.
- If a bless operation is pending, `agd bless --abort` keeps the existing behavior:
  reset the human checkout merge state, remove the persisted bless state, and report
  an aborted operation.

## Verification

- Add a CLI regression test for `agd bless --abort` with no pending bless state.
- Keep existing conflict abort tests passing.
- Run focused tests, formatting, full tests, and lint.
