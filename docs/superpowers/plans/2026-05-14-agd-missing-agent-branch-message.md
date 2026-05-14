# AGD missing agent branch message

## Goal

Replace raw Git fetch errors for missing adoption branches with a clear AGD error.

## Behavior

- `agd bless agent/missing` fails with `agent branch not found: agent/missing`.
- `agd pr --bless agent/missing` reports the same missing-branch error and still
  omits recovery guidance because no bless recovery state exists.
- Existing successful adoption behavior is unchanged.

## Verification

- Strengthen existing missing-branch integration tests.
- Run focused bless/PR tests, formatting, full tests, and lint.
