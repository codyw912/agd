# Workspace List Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd workspace list` so users can inspect the workspaces already recorded in project metadata.

**Architecture:** Keep this slice read-only. Add a nested `workspace list` CLI command, discover the current AGD project from either the human checkout or an agent workspace, and render each workspace's id, name, status, and path. Do not add workspace creation or selection yet.

**Tech Stack:** Rust, clap nested subcommand, existing project metadata, assert_cmd integration test.

---

### Task 1: List Recorded Workspaces

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Add: `src/workspace_commands.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Write failing workspace list test**

Add an integration test that runs `agd workspace list` after init from both the human checkout and default workspace, asserting it prints `default`, `active`, and the workspace path.

- [x] **Step 2: Run focused test to verify failure**

Run: `just dev cargo test workspace_list_shows_recorded_workspaces`

Expected: FAIL because `workspace` is not currently a CLI command.

- [x] **Step 3: Add CLI and dispatch**

Add `Command::Workspace { command: WorkspaceCommand }` with `WorkspaceCommand::List`, and route it through project discovery.

- [x] **Step 4: Implement list rendering**

Create a small workspace command module that iterates `project.workspaces` and prints one line per workspace: `<id>\t<status>\t<path>`.

- [x] **Step 5: Run focused test to verify pass**

Run: `just dev cargo test workspace_list_shows_recorded_workspaces`

Expected: PASS.

- [x] **Step 6: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.
