# AGD Cleanup Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add cleanup commands for deleting unwanted agent branches and rebuilding the default managed workspace.

**Architecture:** Reuse project discovery, workspace metadata, and the existing Git wrapper. Add a focused cleanup module that refuses dirty state before destructive actions, deletes agent branches with Git, and resets the default workspace by removing and recreating the managed clone from the human checkout.

**Tech Stack:** Rust, `clap`, system `git`, existing real-Git integration tests in `tests/p0a.rs`.

---

## File Structure

- `src/cli.rs`: add `Discard { branch }` and `ResetWorkspace`.
- `src/main.rs`: dispatch cleanup commands after AGD project discovery.
- `src/cleanup.rs`: implement clean-state checks, branch deletion, workspace removal, and workspace recreation.
- `tests/p0a.rs`: add integration tests for clean discard, dirty discard refusal, and workspace reset.

## Task 1: Discard Branch

- [ ] Add a failing test that creates `agent/discard-me`, runs `agd discard agent/discard-me`, and asserts the branch is deleted from the agent workspace while the human checkout is unchanged.
- [ ] Run `just dev cargo test discard_deletes_clean_agent_branch`.
- [ ] Implement `agd discard <branch>` by refusing dirty workspace state and running `git branch -D <branch>` in the default workspace.
- [ ] Rerun the test and confirm it passes.

## Task 2: Dirty Discard Refusal

- [ ] Add a failing test that creates uncommitted changes in the workspace and asserts `agd discard` fails with a clear dirty-state message.
- [ ] Run `just dev cargo test discard_refuses_dirty_agent_workspace`.
- [ ] Reuse the cleanup dirty-state check.
- [ ] Rerun the test and confirm it passes.

## Task 3: Reset Workspace

- [ ] Add a failing test that creates an agent branch, runs `agd reset-workspace` from the human checkout, and asserts the workspace exists, the marker exists, guardrails still work, and the old agent branch is gone.
- [ ] Run `just dev cargo test reset_workspace_recreates_clean_managed_clone`.
- [ ] Implement `agd reset-workspace` by refusing dirty workspace state, removing the default workspace directory, recreating it through existing workspace setup, saving metadata, and printing the workspace path.
- [ ] Rerun the test and confirm it passes.

## Task 4: Verification

- [ ] Run `just fmt`.
- [ ] Run `just dev cargo fmt --all -- --check`.
- [ ] Run `just test`.
- [ ] Run `just lint`.
- [ ] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: Covers P0 cleanup behavior for branch discard and workspace reset. This slice chooses safe refusal on dirty workspaces rather than interactive warnings, which is appropriate for the current noninteractive CLI.

Placeholder scan: No placeholder implementation steps remain.

Type consistency: Uses existing `Project`, default workspace metadata, `workspace::ensure_default_workspace`, and system Git wrapper.
