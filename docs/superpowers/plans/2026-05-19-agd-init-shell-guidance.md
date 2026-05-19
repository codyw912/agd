**Goal:** Make `agd init` point users at the existing `agd shell` command as the default way to enter the managed workspace.

**Architecture:** Keep this as an output and documentation improvement. Do not change workspace storage, symlink behavior, or shell semantics in this slice. Preserve `agd path` as the scriptable path lookup.

**Steps:**

1. Add an init-output regression test for the recommended next step.
2. Update human-readable `agd init` output to show `agd shell` after the workspace path.
3. Update the README core workflow to recommend `agd shell` first and `agd path` for scripts or manual `cd`.
4. Run formatting, lint, and tests.
