**Goal:** Document how an external alpha user installs AGD and what limitations they should expect.

**Architecture:** Keep install instructions in the README before the core workflow. Document Rust/cargo installation because no binary release pipeline exists yet. Keep limitations explicit and short so the alpha status is clear without burying the workflow.

**Steps:**

1. Add prerequisites and install commands to the README.
2. Add a known limitations section focused on alpha behavior, external tools, Git LFS/submodules, and sandbox boundaries.
3. Run format and test gates.
