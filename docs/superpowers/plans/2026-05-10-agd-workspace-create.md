# Workspace Create Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add named managed workspace creation and path lookup without changing default-workspace behavior.

**Architecture:** Generalize the existing default workspace creation path so AGD can create additional managed clones under the same project workspace root. `agd workspace create <name>` should clone the human checkout, write a workspace marker with that workspace id, install guardrails, append workspace metadata, and save the project. `agd path --workspace <name>` should return a recorded workspace path while plain `agd path` continues returning the default workspace.

**Tech Stack:** Rust, clap flags/nested commands, system Git, existing workspace metadata and guardrails, assert_cmd integration tests.

---

### Task 1: Create Named Workspaces

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/workspace.rs`
- Modify: `src/workspace_commands.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Write failing workspace create/path test**

Add an integration test that runs `agd workspace create review`, verifies `agd workspace list` includes `review`, verifies `agd path --workspace review` returns the new workspace path, and asserts the created workspace has a marker and guardrails.

- [x] **Step 2: Run focused test to verify failure**

Run: `just dev cargo test workspace_create_adds_named_managed_workspace`

Expected: FAIL because workspace create and `path --workspace` do not exist.

- [x] **Step 3: Add CLI and dispatch**

Add `Path { workspace: Option<String> }` and `WorkspaceCommand::Create { name: String }`. Route create through project discovery and save the mutated project.

- [x] **Step 4: Generalize workspace creation**

Refactor workspace creation helpers to accept a workspace id/name. Keep the existing default helper as a thin wrapper. Ensure markers use the correct workspace id.

- [x] **Step 5: Implement path lookup**

Return the requested recorded workspace path for `agd path --workspace <name>`, and keep existing default path behavior unchanged.

- [x] **Step 6: Run focused test to verify pass**

Run: `just dev cargo test workspace_create_adds_named_managed_workspace`

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
