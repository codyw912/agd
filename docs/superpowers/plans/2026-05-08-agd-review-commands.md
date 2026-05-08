# AGD Review Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add local review commands so a human can inspect agent branches before adoption.

**Architecture:** Reuse the existing project/workspace discovery and system Git wrapper. Add one focused review module that resolves the default workspace and target branch, then shells out to Git for branch listing, commit logs, diffs, and changed files.

**Tech Stack:** Rust, `clap`, system `git`, existing real-Git integration tests in `tests/p0a.rs`.

---

## File Structure

- `src/cli.rs`: add `branches`, `log [branch]`, `diff [branch]`, and `files [branch]` subcommands.
- `src/main.rs`: dispatch review commands after discovering AGD context.
- `src/review.rs`: implement branch/default-branch resolution and Git-backed review output.
- `tests/p0a.rs`: add integration tests using the existing fixture and real temp Git repositories.

## Task 1: Branch Listing

- [ ] Add a failing test that creates `agent/refactor-auth` and asserts `agd branches` prints it.
- [ ] Run `just dev cargo test branches_lists_agent_branches`.
- [ ] Implement `review::branches`.
- [ ] Rerun the test and confirm it passes.

## Task 2: Log/Diff/Files

- [ ] Add a failing test that creates two commits on an agent branch and asserts:
  - `agd log agent/refactor-auth` prints both commit subjects.
  - `agd diff agent/refactor-auth` prints the changed file diff.
  - `agd files agent/refactor-auth` prints the changed file name.
- [ ] Run `just dev cargo test review_commands_show_branch_changes`.
- [ ] Implement `review::log`, `review::diff`, and `review::files`.
- [ ] Rerun the test and confirm it passes.

## Task 3: Defaults From Agent Workspace

- [ ] Add a failing test that runs `agd log`, `agd diff`, and `agd files` from inside the agent workspace without a branch argument and expects the current branch to be used.
- [ ] Run `just dev cargo test review_commands_default_to_current_agent_branch`.
- [ ] Implement current-branch fallback for optional branch arguments.
- [ ] Rerun the test and confirm it passes.

## Task 4: Verification

- [ ] Run `just fmt`.
- [ ] Run `just test`.
- [ ] Run `just lint`.
- [ ] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: Covers the PRD review commands for P0: `agd branches`, `agd log [branch]`, `agd diff [branch]`, and `agd files [branch]`, including command execution from human checkout and default branch resolution from an agent workspace.

Placeholder scan: No placeholder implementation steps remain; deferred JSON output stays outside this slice.

Type consistency: The plan uses existing `Project`, `Workspace`, `ProjectContext`, and system Git wrapper concepts.
