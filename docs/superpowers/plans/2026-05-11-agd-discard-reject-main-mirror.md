# AGD Discard Reject Main Mirror Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent `agd discard` from deleting synced protected mirror branches such as `main`.

**Architecture:** Reuse the conservative protected-branch predicate already used by review and PR branch resolution: a discard target must not be empty, must not equal the configured default target, and must not be `main`. Keep deletion behavior unchanged for real agent branches.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Reject Main Mirror Discard

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/cleanup.rs`

- [x] **Step 1: Write the failing test**

Add `discard_rejects_main_mirror_when_default_target_is_not_main`. Set up a project whose default target is `develop`, advance and sync `main`, run `agd discard main`, and assert it fails with `refusing to discard protected branch` while `main` still resolves in the workspace.

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test discard_rejects_main_mirror_when_default_target_is_not_main`

Expected: FAIL because current `agd discard main` deletes the synced `main` mirror.

- [x] **Step 3: Write minimal implementation**

In `cleanup::discard`, replace the default-target-only check with a helper that rejects `branch.is_empty()`, `branch == project.default_target`, or `branch == "main"` and emits `refusing to discard protected branch`.

- [x] **Step 4: Run focused verification**

Run:

```bash
just dev cargo test discard_
just dev cargo fmt --all -- --check
```

Expected: all discard-focused tests pass and formatting is clean.

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
git add docs/superpowers/plans/2026-05-11-agd-discard-reject-main-mirror.md src/cleanup.rs tests/p0a.rs
git commit -m "fix: reject main mirror discard"
```
