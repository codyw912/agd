# Doctor Workspace Marker Drift Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor` detect workspace marker metadata drift and confirm `agd doctor --repair` rewrites the marker.

**Architecture:** Replace the current marker existence-only doctor check with a validation check that parses `.agd/workspace.json` and compares `kind`, `project_id`, `workspace_id`, and `human_checkout` to project metadata. Reuse the existing `doctor --repair` marker rewrite path for repair.

**Tech Stack:** Rust, serde_json, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Detect Workspace Marker Drift

**Files:**
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [ ] **Step 1: Write the failing test**

Add `doctor_detects_and_repairs_workspace_marker_drift` to `tests/p0a.rs`. Initialize AGD, overwrite `.agd/workspace.json` with JSON containing the wrong `project_id` and `workspace_id`, run `agd doctor`, and assert it fails on `workspace marker`. Run `agd doctor --repair`, then assert `agd doctor` succeeds and the marker has the correct `project_id` and `workspace_id`.

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_detects_and_repairs_workspace_marker_drift`

Expected: FAIL because current doctor only checks that `.agd/workspace.json` exists.

- [ ] **Step 3: Implement minimal validation**

In `src/doctor.rs`, replace the `path_check("workspace marker", ...)` call with `workspace_marker_check(project, workspace)`. The helper should parse `workspace::WorkspaceMarker`, compare marker fields to the expected project/workspace values, and return a failed check with a concise drift detail on mismatch.

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test doctor_detects_and_repairs_workspace_marker_drift`

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
git add docs/superpowers/plans/2026-05-09-agd-doctor-workspace-marker-drift.md src/doctor.rs tests/p0a.rs
git commit -m "feat: detect workspace marker metadata drift"
```
