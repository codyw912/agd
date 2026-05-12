# AGD Status Branch Summaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd status` more useful by showing pending agent branches with commit and changed-file counts, matching the PRD status examples.

**Architecture:** Add a small status summary helper that reuses the existing protected-branch-aware review branch selection. For each candidate branch, compute commits unique to the default target and changed files relative to the default target. Use the same helper in human-readable and JSON status output so both surfaces stay consistent.

**Tech Stack:** Rust CLI, system Git subprocesses, serde JSON output, real-Git integration tests, `just` project commands.

---

### Task 1: Status Branch Summaries

**Files:**
- Add: `src/status_report.rs`
- Modify: `src/main.rs`
- Modify: `src/output.rs`
- Modify: `src/json_output.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Add failing human status test**

Add a test that creates two candidate agent branches and asserts human-readable `agd status` prints a `Pending agent branches:` section with branch names, unique commit counts, and changed-file counts.

- [x] **Step 2: Add failing JSON status test**

Extend JSON status coverage to assert an `agent_branches` array with `{ branch, commits, files_changed }` entries for the same summary data.

- [x] **Step 3: Run tests to verify they fail**

Run:

```bash
just dev cargo test status_shows_pending_agent_branch_summaries
just dev cargo test json_status_outputs_agent_branch_summaries
```

Expected: FAIL because status currently reports mode and metadata only.

- [x] **Step 4: Implement shared status summary helper**

Add a shared helper that returns protected-branch-filtered branch summaries using the default workspace and default target.

- [x] **Step 5: Wire human and JSON output**

Print the pending branch summary in `agd status` and include `agent_branches` in JSON status output.

- [x] **Step 6: Run focused verification**

Run:

```bash
just dev cargo test status_shows_pending_agent_branch_summaries
just dev cargo test json_status_outputs_agent_branch_summaries
just dev cargo test status_identifies_human_checkout_and_agent_workspace
just dev cargo test json_path_and_status_outputs_are_machine_readable
just dev cargo fmt --all -- --check
```

Expected: focused status tests pass and formatting is clean.

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
git add docs/superpowers/plans/2026-05-12-agd-status-branch-summaries.md src/status_report.rs src/main.rs src/output.rs src/json_output.rs tests/p0a.rs
git commit -m "feat: summarize agent branches in status"
```
