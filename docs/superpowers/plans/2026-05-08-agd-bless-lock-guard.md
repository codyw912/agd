# Bless Lock Guard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make an existing `bless.lock` stop adoption with a clear AGD error before any adoption mutation.

**Architecture:** Keep the existing atomic `create_new` lock acquisition. Special-case `AlreadyExists` into a user-facing `bless operation already in progress` error and preserve the current lock cleanup behavior for locks created by this process.

**Tech Stack:** Rust, anyhow, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Existing Lock Error

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/adoption.rs`

- [ ] **Step 1: Write the failing test**

Add an integration test that initializes AGD, creates an agent branch, manually writes `projects/<project_id>/locks/bless.lock`, runs `agd bless agent/refactor-auth`, and asserts stderr contains `bless operation already in progress` and `bless.lock`. Assert the human checkout still has one commit.

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test bless_refuses_existing_operation_lock`

Expected: FAIL because the current stderr reports a low-level `create lock ...` context instead of the clear AGD message.

- [ ] **Step 3: Implement minimal lock collision handling**

In `src/adoption.rs`, match on `OpenOptions::open`. If the error kind is `AlreadyExists`, return `anyhow::bail!("bless operation already in progress: {}", path.display())`. Keep the existing contextual error for other I/O errors.

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test bless_refuses_existing_operation_lock`

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
git add docs/superpowers/plans/2026-05-08-agd-bless-lock-guard.md tests/p0a.rs src/adoption.rs
git commit -m "feat: improve bless operation lock guard"
```
