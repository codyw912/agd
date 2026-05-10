# AGD JSON Identity Output

## Goal

Add structured output for `agd --json identity` so plugins and scripts do not need to parse the human-readable identity output.

## Scope

- Keep existing `agd identity` text output unchanged.
- Return agent identity fields as JSON when `--json` is supplied.
- Reuse the same field names already used in JSON status output.

## Test

- Assert `agd --json identity` returns `name`, `email`, and `signing`.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
