**Goal:** Add release metadata needed before tagging AGD's first alpha.

**Architecture:** Keep this docs/package-only. Add package repository/readme metadata, create a changelog with the initial alpha scope, and require changelog review in the release checklist.

**Steps:**

1. Add Cargo package metadata for repository and README.
2. Add `CHANGELOG.md` with an unreleased initial alpha entry.
3. Update the release checklist to require changelog review.
4. Run format, test, and lint gates.
