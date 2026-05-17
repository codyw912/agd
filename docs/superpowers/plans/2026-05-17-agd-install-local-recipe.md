**Goal:** Add a `just` wrapper for installing the current AGD checkout as the user-visible `agd` binary and record the workspace-entry ergonomics observation from dogfooding setup.

**Architecture:** Keep the wrapper in `justfile` and explicitly set `CARGO_HOME` and `CARGO_INSTALL_ROOT` to `~/.cargo` so the install does not land inside the project devenv cargo-install directory. Document the recipe in the README development command list. Keep the workspace-entry ergonomics note observation-only.

**Steps:**

1. Add `just install-local`.
2. Document it in README development commands.
3. Record the `cd "$(agd path)"` ergonomics observation.
4. Verify it installs `agd` into `~/.cargo/bin`.
