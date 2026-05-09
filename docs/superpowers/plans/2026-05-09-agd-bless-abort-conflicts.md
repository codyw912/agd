# Bless Abort Conflict Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a clean abort path for conflicted squash adoption with `agd bless --abort`.

**Architecture:** Keep conflict recovery Git-native for this slice. Squash adoption conflicts leave the human checkout with unmerged index entries but no `MERGE_HEAD`, so `git merge --abort` is not sufficient. Use `git reset --merge` from the human checkout to restore the pre-adoption HEAD and index. Add CLI validation so `agd bless --abort` is mutually exclusive with branch adoption flags. Leave full `agd bless --continue` for a later slice because it requires persisted adoption metadata and commit-message reconstruction.

**Tech Stack:** Rust, system Git, clap CLI parsing, assert_cmd integration tests.

---

### Task 1: Add Squash Bless Abort

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/adoption.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Write failing conflict-abort test**

Add an integration test that creates conflicting human and agent changes, runs `agd bless <branch>` and expects failure with `agd bless --abort`, then runs `agd bless --abort` and asserts the human checkout is clean and still at the human commit.

- [x] **Step 2: Run focused test to verify failure**

Run: `just dev cargo test bless_abort_restores_human_checkout_after_squash_conflict`

Expected: FAIL because `--abort` is not currently accepted.

- [x] **Step 3: Add CLI shape and dispatch**

Make `Bless.branch` optional and add `--abort`. In `main`, route abort to `adoption::abort(project)` and validate that normal adoption still requires a branch.

- [x] **Step 4: Implement abort**

In `adoption.rs`, add `abort(project)` that runs `git reset --merge` in the human checkout and prints `Aborted bless operation`.

- [x] **Step 5: Improve squash conflict guidance**

When `git merge --squash` fails, include `agd bless --abort` in the error so users have a clear recovery path.

- [x] **Step 6: Run focused test to verify pass**

Run: `just dev cargo test bless_abort_restores_human_checkout_after_squash_conflict`

Expected: PASS.

- [x] **Step 7: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.
