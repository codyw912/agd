# Bless Lock Metadata Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record PRD-required recovery metadata in `bless.lock` while an adoption operation is active.

**Architecture:** Keep the existing `AdoptionLock` ownership and cleanup model. Pass the default workspace id into lock acquisition, add `workspace_id`, `pid`, and RFC3339 `started_at` fields to the JSON, and verify the file while `bless` is still running by capturing it from the test signing hook.

**Tech Stack:** Rust, anyhow, time, serde_json, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Bless Lock Recovery Metadata

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/adoption.rs`

- [ ] **Step 1: Write the failing test**

Add `bless_lock_records_recovery_metadata_while_operation_runs` to `tests/p0a.rs`. Configure a fake human signer that copies `AGD_TEST_LOCK_PATH` to `AGD_TEST_CAPTURE_PATH` before emitting a fake signature. Run `agd bless agent/refactor-auth` with those env vars, parse the captured JSON, and assert it contains `operation_id`, `operation`, `project_id`, `workspace_id`, `pid`, and parseable `started_at`.

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test bless_lock_records_recovery_metadata_while_operation_runs`

Expected: FAIL because the current `bless.lock` has no `workspace_id`, `pid`, or `started_at` fields.

- [ ] **Step 3: Write minimal implementation**

Change `AdoptionLock::acquire` to accept `workspace_id`. Include these fields in the lock JSON:

```rust
"workspace_id": workspace_id,
"pid": std::process::id(),
"started_at": time::OffsetDateTime::now_utc()
    .format(&time::format_description::well_known::Rfc3339)
    .context("format lock timestamp")?,
```

Update the call from `prepare` to pass `&workspace.id`.

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test bless_lock_records_recovery_metadata_while_operation_runs`

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
git add docs/superpowers/plans/2026-05-08-agd-bless-lock-metadata.md tests/p0a.rs src/adoption.rs
git commit -m "feat: record bless lock recovery metadata"
```
