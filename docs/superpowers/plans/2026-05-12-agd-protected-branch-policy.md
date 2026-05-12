# AGD Protected Branch Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply AGD's protected-by-default branch policy consistently across review, PR, sync rebase, and cleanup commands.

**Architecture:** Introduce one small branch policy module containing the PRD's default protected branch patterns: `main`, `master`, `trunk`, `develop`, `release/**`, `stable/**`, `production/**`, and `prod/**`, plus the project default target. Use it anywhere AGD validates "agent branch required" or refuses branch deletion. Keep normal Git branch creation in agent workspaces unchanged; the pre-commit hook remains the direct-commit guardrail.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Default Protected Branch Policy

**Files:**
- Add: `src/branch_policy.rs`
- Modify: `src/main.rs`
- Modify: `src/guardrails.rs`
- Modify: `src/review.rs`
- Modify: `src/pull_request.rs`
- Modify: `src/sync.rs`
- Modify: `src/cleanup.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Add failing review/list tests**

Add tests that protected default branch names and prefixes are omitted from `agd branches` and rejected by explicit review commands, while non-`agent/` work branches like `scratch/experiment` remain allowed.

- [x] **Step 2: Add failing mutator tests**

Add tests that `agd pr`, `agd sync --rebase`, and `agd discard --force` reject default protected branch patterns such as `release/1.0`.

- [x] **Step 3: Run tests to verify they fail**

Run:

```bash
just dev cargo test branches_omits_default_protected_branch_patterns
just dev cargo test review_commands_reject_default_protected_branch_patterns
just dev cargo test pr_rejects_default_protected_branch_patterns
just dev cargo test sync_rebase_rejects_default_protected_branch_patterns
just dev cargo test discard_force_rejects_default_protected_branch_patterns
```

Expected: FAIL because AGD currently rejects only the project default target and `main` in several command paths.

- [x] **Step 4: Implement central policy**

Add `branch_policy` helpers for protected/default-agent branch validation and switch existing command-specific checks to them.

- [x] **Step 5: Keep hook policy aligned**

Make the protected-branch pre-commit hook derive its shell `case` pattern from the central policy constants so future policy changes touch one source.

- [x] **Step 6: Run focused verification**

Run:

```bash
just dev cargo test branches_omits_default_protected_branch_patterns
just dev cargo test review_commands_reject_default_protected_branch_patterns
just dev cargo test pr_rejects_default_protected_branch_patterns
just dev cargo test sync_rebase_rejects_default_protected_branch_patterns
just dev cargo test discard_force_rejects_default_protected_branch_patterns
just dev cargo test agent_workspace_blocks_commits_on_protected_branches
just dev cargo test pr_rejects_main_mirror_when_default_target_is_not_main
just dev cargo test sync_rebase_rejects_protected_branch_before_syncing
just dev cargo fmt --all -- --check
```

Expected: protected policy tests pass and formatting is clean.

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
git add docs/superpowers/plans/2026-05-12-agd-protected-branch-policy.md src/branch_policy.rs src/main.rs src/guardrails.rs src/review.rs src/pull_request.rs src/sync.rs src/cleanup.rs tests/p0a.rs
git commit -m "fix: apply protected branch policy consistently"
```
