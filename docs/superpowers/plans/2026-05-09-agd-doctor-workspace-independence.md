# Doctor Workspace Independence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor` detect managed workspaces that depend on Git object alternates instead of being independent clones.

**Architecture:** Add a doctor check after workspace existence/marker validation. The check inspects the workspace Git metadata for `objects/info/alternates` and fails when the file exists with non-empty content. This keeps the default AGD workspace contract explicit without changing workspace creation.

**Tech Stack:** Rust, system Git wrapper, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Detect Workspace Alternates

**Files:**
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [ ] **Step 1: Write the failing test**

Add `doctor_detects_workspace_object_alternates` to `tests/p0a.rs`. Initialize AGD, write a non-empty `.git/objects/info/alternates` file in the workspace, run `agd doctor`, and assert it fails with `workspace independence` and `alternates`.

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_detects_workspace_object_alternates`

Expected: FAIL because current doctor does not check object alternates.

- [ ] **Step 3: Implement minimal check**

In `src/doctor.rs`, add `workspace_independence_check(&workspace.path)` and call it after `workspace_marker_check`. The helper should resolve `objects/info/alternates` through `git rev-parse --git-path`, read it if present, and fail when it contains non-whitespace content.

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test doctor_detects_workspace_object_alternates`

Expected: PASS.

- [ ] **Step 5: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/plans/2026-05-09-agd-doctor-workspace-independence.md src/doctor.rs tests/p0a.rs
git commit -m "feat: detect workspace object alternates"
```
