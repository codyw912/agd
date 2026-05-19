**Goal:** Make `agd pr --bless` support the same adoption mode flags documented for `agd bless`.

**Architecture:** Extend only the blessed PR path. Add `--preserve` and `--merge` to `agd pr --bless`, pass the selected `AdoptionMode` into `pull_request::open_blessed`, and reject mode flags on raw `agd pr`. Keep squash as the default.

**Steps:**

1. Add regression coverage for `agd pr --bless --preserve`, `agd pr --bless --merge`, and invalid PR mode flag combinations.
2. Extend the CLI and PR adoption path to accept an adoption mode.
3. Update README command synopsis.
4. Run focused tests, formatting, lint, and the full test suite.
