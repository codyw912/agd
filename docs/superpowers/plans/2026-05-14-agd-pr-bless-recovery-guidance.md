# AGD PR bless recovery guidance only when recoverable

## Goal

Avoid telling users to run `agd pr --continue` or `agd bless --abort` when
`agd pr --bless` fails before AGD has created any bless recovery state.

## Behavior

- Early adoption failures, such as a missing agent branch, report the underlying
  error without recovery commands that cannot do anything.
- Recoverable adoption failures, such as signing failures after bless state is
  written, keep the existing `agd pr --continue` and `agd bless --abort`
  guidance.

## Verification

- Add an integration test for `agd pr --bless agent/missing`.
- Keep the existing signing-failure recovery guidance test passing.
- Run focused PR tests, formatting, full tests, and lint.
