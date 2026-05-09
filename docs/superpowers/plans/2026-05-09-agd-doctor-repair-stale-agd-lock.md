# Doctor Repair Stale AGD Lock Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor --repair` remove stale AGD operation locks when lock metadata proves the owning process is gone.

**Architecture:** Keep `agd doctor` conservative: any `bless.lock` still fails the operation-lock check. Add a repair-only path that reads `locks/bless.lock`, requires a numeric `pid`, checks whether that process is alive, and removes the lock only when the process is not alive.

**Tech Stack:** Rust, serde_json, system process check via `kill -0` on macOS/Linux, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Repair Stale AGD Operation Lock

**Files:**
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [ ] **Step 1: Write the failing test**

Add `doctor_repair_removes_stale_agd_operation_lock` to `tests/p0a.rs`. Initialize AGD, write `projects/<project_id>/locks/bless.lock` with a high non-existent `pid`, run `agd doctor` and assert it fails on `AGD operation lock`, run `agd doctor --repair`, then assert:

- stdout contains `Removed stale AGD operation lock`
- the lock file no longer exists
- `agd doctor` succeeds

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_repair_removes_stale_agd_operation_lock`

Expected: FAIL because current `doctor --repair` does not remove stale AGD locks.

- [ ] **Step 3: Implement minimal repair**

In `src/doctor.rs`, add `repair_stale_agd_lock(paths, &project)` and call it from `repair`. The helper should:

- read `locks/bless.lock`
- parse JSON
- require numeric `pid`
- return without removing if the pid is alive
- remove the lock and print `Removed stale AGD operation lock: <path>` when the pid is not alive

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test doctor_repair_removes_stale_agd_operation_lock`

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
git add docs/superpowers/plans/2026-05-09-agd-doctor-repair-stale-agd-lock.md src/doctor.rs tests/p0a.rs
git commit -m "feat: repair stale AGD operation locks"
```
