# AGD Cleanup Workspace Targets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let cleanup commands operate on named workspaces, not only the default workspace.

**Architecture:** Add `--workspace <id>` to `agd discard` and `agd reset-workspace`. Resolve the workspace id through project metadata, defaulting to the project default workspace when omitted. Keep branch protection, dirty-state refusal, force semantics, lock metadata, and JSON output behavior consistent with the existing default-workspace implementation. For reset, recreate the selected workspace with the existing workspace creation path so marker and guardrails are restored.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Named Workspace Cleanup

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/cleanup.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Add failing named discard test**

Add a test that creates a `review` workspace, creates a branch only there, runs `agd discard --workspace review <branch>`, and asserts the branch is deleted from `review` while default workspace branches are untouched.

- [x] **Step 2: Add failing named reset test**

Add a test that creates a `review` workspace, dirties it, runs `agd reset-workspace --workspace review --force`, and asserts only `review` is recreated while the default workspace remains untouched.

- [x] **Step 3: Run tests to verify they fail**

Run:

```bash
just dev cargo test discard_targets_named_workspace
just dev cargo test reset_workspace_targets_named_workspace
```

Expected: FAIL because cleanup commands do not yet accept `--workspace`.

- [x] **Step 4: Implement CLI and dispatch plumbing**

Add optional `workspace` flags to discard/reset-workspace and pass them into cleanup functions.

- [x] **Step 5: Implement workspace resolution**

Resolve the requested workspace id, acquire locks against that workspace, and recreate the same workspace id during reset.

- [x] **Step 6: Run focused verification**

Run:

```bash
just dev cargo test discard_targets_named_workspace
just dev cargo test reset_workspace_targets_named_workspace
just dev cargo test json_discard_outputs_deleted_branch
just dev cargo test json_reset_workspace_outputs_recreated_workspace
just dev cargo test discard_force_deletes_dirty_agent_branch
just dev cargo fmt --all -- --check
```

Expected: named cleanup and existing default cleanup behavior pass.

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
git add docs/superpowers/plans/2026-05-12-agd-cleanup-workspace-targets.md src/cli.rs src/main.rs src/cleanup.rs tests/p0a.rs
git commit -m "feat: target cleanup by workspace"
```
