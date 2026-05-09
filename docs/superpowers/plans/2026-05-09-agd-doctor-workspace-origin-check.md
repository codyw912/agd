# Doctor Workspace Origin Check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor` detect when the managed workspace fetch origin no longer points at the human checkout.

**Architecture:** Add a doctor check that reads `remote.origin.url` from the default workspace and compares it to the project human checkout path using the existing path-equivalence helper. Keep repair simple: the existing `doctor --repair` path already repoints workspace origin to the current human checkout.

**Tech Stack:** Rust, system Git wrapper, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Workspace Origin Detection

**Files:**
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [ ] **Step 1: Write the failing test**

Add `doctor_detects_and_repairs_broken_workspace_origin` to `tests/p0a.rs`. Initialize AGD, change the workspace `remote.origin.url` to a stale path, run `agd doctor`, and assert it fails with `fail workspace origin`. Then run `agd doctor --repair`, run `agd doctor` again, and assert the workspace origin equals the current human checkout root.

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_detects_and_repairs_broken_workspace_origin`

Expected: FAIL because current doctor does not check `remote.origin.url`.

- [ ] **Step 3: Implement minimal check**

In `src/doctor.rs`, add `workspace_origin_check(project, &workspace.path)`. It should:

- read `remote.origin.url`
- treat it as a path
- return ok when it is equivalent to `project.human_checkout`
- fail with the expected and actual values otherwise

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test doctor_detects_and_repairs_broken_workspace_origin`

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
git add docs/superpowers/plans/2026-05-09-agd-doctor-workspace-origin-check.md src/doctor.rs tests/p0a.rs
git commit -m "feat: detect broken workspace origin"
```
