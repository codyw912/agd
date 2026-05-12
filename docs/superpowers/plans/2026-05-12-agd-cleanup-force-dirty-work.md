# AGD Cleanup Force Dirty Work Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users explicitly discard dirty agent cleanup targets while preserving safe default refusal.

**Architecture:** Add `--force` to `agd discard <branch>` and `agd reset-workspace`. Without `--force`, behavior remains unchanged. With `--force`, AGD may discard uncommitted workspace changes before deleting an agent branch or recreating the workspace. Protected branches remain protected even under `--force`.

**Tech Stack:** Rust CLI, system Git subprocesses, cleanup module, real-Git integration tests, `just` project commands.

---

### Task 1: Force Dirty Cleanup

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/cleanup.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Add failing force cleanup tests**

Add integration tests for:

- `discard_force_deletes_dirty_agent_branch`
- `discard_force_still_rejects_protected_branch`
- `reset_workspace_force_recreates_dirty_workspace`

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
just dev cargo test discard_force_deletes_dirty_agent_branch
just dev cargo test discard_force_still_rejects_protected_branch
just dev cargo test reset_workspace_force_recreates_dirty_workspace
```

Expected: FAIL because the CLI does not yet accept `--force`.

- [x] **Step 3: Implement force flags**

Add `force: bool` to `Discard` and `ResetWorkspace`, pass it into cleanup functions, keep protected branch rejection before dirty cleanup, and skip clean-state refusal only when force is true.

- [x] **Step 4: Implement forced branch deletion**

When forced discard targets the currently checked-out branch, reset the workspace hard, switch to the project default target, then delete the target branch. For forced cleanup on another branch, reset dirty workspace state before deletion.

- [x] **Step 5: Run focused verification**

Run:

```bash
just dev cargo test discard_
just dev cargo test reset_workspace_
just dev cargo test json_discard_outputs_deleted_branch
just dev cargo test json_reset_workspace_outputs_recreated_workspace
just dev cargo fmt --all -- --check
```

Expected: cleanup-focused tests pass and formatting is clean.

- [x] **Step 6: Run full verification**

Run:

```bash
just fmt
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all verification commands pass.

- [x] **Step 7: Commit**

```bash
git add docs/superpowers/plans/2026-05-12-agd-cleanup-force-dirty-work.md src/cli.rs src/main.rs src/cleanup.rs tests/p0a.rs
git commit -m "feat: force dirty cleanup"
```
