# AGD bless validates target before AGD refs

## Goal

Make branch adoption validate an explicit target branch before writing AGD refs in
the human checkout.

## Behavior

- `agd bless <branch> --target <missing>` fails before fetching the agent branch
  into `refs/agd/agent/*`.
- The same failure also happens before writing `refs/agd/safety/*`.
- Valid target branch adoption keeps the existing behavior.

## Verification

- Add an integration regression test for a missing `--target`.
- Keep existing explicit-target adoption tests passing.
- Run focused tests, formatting, full tests, and lint.
