# AGD Sync Rebase Agent Branch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd sync --rebase <agent-branch>` so users can explicitly update mirror branches from the human checkout and then rebase a selected agent branch onto the updated default target.

**Architecture:** Keep plain `agd sync` behavior unchanged. Add an optional `--rebase` branch argument to the sync command, reject protected mirror branches (`main` and the configured default target), run the existing fast-forward mirror sync first, then run `git rebase <default-target> <agent-branch>` in the managed workspace. Return optional rebase metadata in JSON and print a second human line when a branch was rebased.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Sync And Rebase An Agent Branch

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/sync.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Add failing text-output behavior test**

Add `sync_rebases_selected_agent_branch_after_mirror_update`. It should create an agent branch, advance the human default target, run `agd sync --rebase agent/rebase-me`, and assert:

- stdout contains `Synced main`
- stdout contains `Rebased agent/rebase-me`
- workspace `main` matches human `main`
- rebased agent branch descends from the updated `main`
- agent branch tip changed

- [x] **Step 2: Implement minimal text behavior**

Add the CLI option, thread it through `main`, update `sync::sync`, reject protected rebase targets, run the rebase after mirror updates, and print `Rebased <branch>`.

- [x] **Step 3: Add JSON behavior test**

Extend coverage with `agd --json sync --rebase agent/rebase-json`, asserting the response includes `rebase.branch`, `rebase.before`, and `rebase.after`.

- [x] **Step 4: Implement JSON result shape**

Add an optional `rebase` object to `SyncResult` with branch, before, and after commit ids.

- [x] **Step 5: Add protected-branch rejection test**

Add coverage that `agd sync --rebase main` fails with `agent branch is required` and leaves `main` as a mirror branch only.

- [x] **Step 6: Run focused verification**

Run:

```bash
just dev cargo test sync_
just dev cargo fmt --all -- --check
```

Expected: sync-focused tests pass and formatting is clean.

- [x] **Step 7: Run full verification**

Run:

```bash
just fmt
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all verification commands pass.

- [x] **Step 8: Commit**

```bash
git add docs/superpowers/plans/2026-05-12-agd-sync-rebase-agent-branch.md src/cli.rs src/main.rs src/sync.rs tests/p0a.rs
git commit -m "feat: rebase agent branch during sync"
```
