# AGD JSON Init Output

## Goal

Add structured output for `agd --json init` so setup automation can initialize AGD and consume the created project/workspace metadata without parsing human text.

## Scope

- Keep existing `agd init` text output unchanged.
- Return project identity and paths when `--json` is supplied.
- Include the default workspace record using the same workspace fields as other JSON responses.

## Test

- Assert `agd --json init` returns `project_id`, `human_checkout`, and a default workspace object.
- Assert the reported workspace path exists.

## Status

- [x] Failing test added
- [x] Command implemented
- [x] Verification run
