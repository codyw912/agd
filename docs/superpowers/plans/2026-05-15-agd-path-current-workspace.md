# AGD path defaults to current workspace

## Goal

Make `agd path` consistent with other workspace-aware commands when it is run
from inside a named agent workspace.

## Behavior

- `agd path --workspace <id>` keeps returning the explicit workspace path.
- `agd path` from the human checkout keeps returning the default workspace path.
- `agd path` from inside an agent workspace returns that current workspace path.
- JSON output follows the same selection rule.

## Verification

- Add integration coverage for text and JSON `agd path` from a named workspace.
- Keep existing path tests passing.
- Run focused path tests, formatting, full tests, and lint.
