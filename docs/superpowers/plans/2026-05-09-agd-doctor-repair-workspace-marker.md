# Doctor Repair Workspace Marker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor --repair` recreate a missing managed workspace marker from project metadata.

**Architecture:** Expose the existing workspace marker writer through a small repair helper in `workspace.rs`. During `doctor --repair`, reuse that helper for each existing managed workspace after path/origin repair and guardrail reinstall, then verify `agd doctor` returns healthy.

**Tech Stack:** Rust, serde_json, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Repair Missing Workspace Marker

**Files:**
- Modify: `src/workspace.rs`
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [ ] **Step 1: Write the failing test**

Add `doctor_repair_recreates_missing_workspace_marker` to `tests/p0a.rs`. Initialize AGD, delete `.agd/workspace.json`, run `agd doctor` and assert `fail workspace marker`, run `agd doctor --repair`, then assert:

- stdout contains `Repaired workspace marker`
- `agd doctor` succeeds
- recreated marker has `kind=agd-workspace`
- marker has the project id and `workspace_id=default`
- marker `human_checkout` matches the human checkout

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_repair_recreates_missing_workspace_marker`

Expected: FAIL because current `doctor --repair` reinstalls guardrails but does not recreate `.agd/workspace.json`.

- [ ] **Step 3: Implement minimal repair**

In `src/workspace.rs`, add `pub fn repair_workspace_marker(project: &Project, workspace: &Workspace) -> Result<()>` that calls the existing marker writer with `workspace.created_at`.

In `src/doctor.rs`, import `crate::workspace` and call `workspace::repair_workspace_marker(&project, workspace)?` for each existing workspace. Print `Repaired workspace marker for <workspace-id>`.

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test doctor_repair_recreates_missing_workspace_marker`

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
git add docs/superpowers/plans/2026-05-09-agd-doctor-repair-workspace-marker.md src/workspace.rs src/doctor.rs tests/p0a.rs
git commit -m "feat: repair missing workspace marker"
```
