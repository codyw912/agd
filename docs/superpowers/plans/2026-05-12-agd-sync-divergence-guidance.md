# AGD Sync Divergence Guidance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When `agd sync` refuses a non-fast-forward mirror update, include an explicit recovery command suggestion.

**Architecture:** Keep sync behavior unchanged: mirror branches still fast-forward only and `agent/*` branches remain untouched. Tighten the existing divergence error text in `src/sync.rs` so it explains the refusal and points users at `agd reset-workspace` for an explicit repair path.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Add Recovery Guidance To Sync Divergence Errors

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/sync.rs`

- [x] **Step 1: Write the failing test**

Extend `sync_refuses_dirty_or_diverged_state` so the diverged-default-target case still expects failure, but also requires stderr to contain `agd reset-workspace`.

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test sync_refuses_dirty_or_diverged_state`

Expected: FAIL because the current divergence error says only that the target cannot be fast-forwarded.

- [x] **Step 3: Write minimal implementation**

Update `ensure_fast_forward` to include `Run \`agd reset-workspace\` to recreate the managed workspace from the human checkout.` in both default-target and named-target divergence errors.

- [x] **Step 4: Run focused verification**

Run:

```bash
just dev cargo test sync_refuses_dirty_or_diverged_state
just dev cargo test sync_
just dev cargo fmt --all -- --check
```

Expected: sync-focused tests pass and formatting is clean.

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
git add docs/superpowers/plans/2026-05-12-agd-sync-divergence-guidance.md src/sync.rs tests/p0a.rs
git commit -m "fix: suggest reset after sync divergence"
```
