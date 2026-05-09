# Operation Locks for Mutating Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make AGD mutating commands share the same project operation lock behavior instead of locking only `bless`.

**Architecture:** Extract the current bless lock into a shared operation-lock helper. The helper creates `projects/<project-id>/locks/<operation>.lock` with the existing metadata shape, refuses when any project lock already exists, and removes locks it created on success/failure. Commands that mutate project or workspace state acquire the lock after clean-state validation and before mutation. Doctor checks and stale-lock repair scan the whole locks directory.

**Tech Stack:** Rust, system Git integration tests, JSON lock metadata, `just` verification recipes.

---

### Task 1: Share Operation Locking Across Mutators

**Files:**
- Add: `src/operation_lock.rs`
- Modify: `src/main.rs`
- Modify: `src/adoption.rs`
- Modify: `src/sync.rs`
- Modify: `src/cleanup.rs`
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Write failing lock tests**

Add integration coverage showing that existing AGD operation locks block `sync`, `discard`, and `reset-workspace`, and that `doctor` reports non-bless lock files.

- [x] **Step 2: Run focused tests to verify failure**

Run: `just dev cargo test operation_locks_block_mutating_commands`

Expected: FAIL because only `bless` currently checks operation locks.

- [x] **Step 3: Extract shared operation lock helper**

Move the bless lock metadata/write/drop behavior into `src/operation_lock.rs`. Keep `bless.lock` path and metadata unchanged for existing behavior.

- [x] **Step 4: Acquire locks from mutating commands**

Update `sync`, `discard`, and `reset-workspace` to acquire operation locks before their mutation points. Keep clean-state failures ahead of lock acquisition where practical.

- [x] **Step 5: Generalize doctor lock scanning**

Update doctor detection and stale repair to scan all `*.lock` files in the project locks directory.

- [x] **Step 6: Run focused tests to verify pass**

Run: `just dev cargo test operation_locks_block_mutating_commands`

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
