# AGD Init Repository Completeness Warnings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd init` warn immediately when the source repository uses submodules or Git LFS, so setup does not silently create a workspace that may need extra repository data.

**Architecture:** Reuse existing doctor warning checks after init has created and saved project metadata. Print only warning checks to stderr, leaving normal stdout and JSON stdout stable. Keep init successful: this is detection and user guidance, not blocking.

**Tech Stack:** Rust CLI, existing doctor checks, real-Git integration tests, `just` project commands.

---

### Task 1: Emit Init Warnings For Repository Completeness Risks

**Files:**
- Modify: `src/doctor.rs`
- Modify: `src/main.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Add failing init warning tests**

Add two integration tests:

- `init_warns_when_submodules_are_declared`
- `json_init_warns_on_lfs_without_polluting_stdout`

The tests should assert init still succeeds, emits warning text on stderr, and JSON init stdout remains parseable.

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
just dev cargo test init_warns_when_submodules_are_declared
just dev cargo test json_init_warns_on_lfs_without_polluting_stdout
```

Expected: FAIL because init currently prints no repository completeness warnings.

- [x] **Step 3: Implement warning emission**

Add a doctor helper that prints only warning checks to stderr. After `agd init` saves project metadata, build a human-checkout project context, run `doctor::report`, and print warnings. Do this before JSON output so stdout remains clean and stderr carries warnings.

- [x] **Step 4: Run focused verification**

Run:

```bash
just dev cargo test init_
just dev cargo test json_init_outputs_project_and_workspace_metadata
just dev cargo fmt --all -- --check
```

Expected: init-focused tests pass and formatting is clean.

- [x] **Step 5: Run full verification**

Run:

```bash
just fmt
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all verification commands pass.

- [x] **Step 6: Commit**

```bash
git add docs/superpowers/plans/2026-05-12-agd-init-repo-completeness-warnings.md src/doctor.rs src/main.rs tests/p0a.rs
git commit -m "feat: warn on repository completeness risks at init"
```
