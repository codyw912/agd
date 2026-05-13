# AGD Bless Adoption Branches Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd bless <agent-branch>` adopt agent work onto a human-owned adoption branch by default instead of mutating the target branch directly.

**Architecture:** Keep `bless` as the primitive that turns agent-owned commits into human-owned history. Add branch selection options in the CLI, have the adoption layer create/switch the human checkout to the adoption branch before applying the existing squash/preserve/merge modes, and keep the old current-branch behavior behind an explicit `--direct` flag. Derive the default adoption branch by stripping an `agent/` prefix from the agent branch.

**Tech Stack:** Rust CLI with clap, existing Git subprocess helpers, integration tests in `tests/p0a.rs`.

---

### Task 1: Default Adoption Branch

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/adoption.rs`

- [x] **Step 1: Write the failing test**

Add an integration test proving `agd bless agent/refactor-auth` creates `refactor-auth`, switches the human checkout to it, and leaves `main` unchanged.

- [x] **Step 2: Run the test to verify it fails**

Run: `just dev cargo test bless_defaults_to_human_adoption_branch`

Expected: FAIL because current `bless` commits directly on `main`.

- [x] **Step 3: Implement branch-targeted bless**

Add adoption target options, derive the default branch by stripping `agent/`, refuse branch collisions, switch to a new branch from the selected target branch, and run the existing adoption mode there.

- [x] **Step 4: Run the test to verify it passes**

Run: `just dev cargo test bless_defaults_to_human_adoption_branch`

Expected: PASS.

### Task 2: Overrides And Compatibility

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/adoption.rs`
- Modify: `README.md`

- [x] **Step 1: Add coverage for `--branch`, `--target`, and `--direct`**

Add focused integration tests for explicit adoption branch naming, custom base target, collision refusal, and the legacy direct adoption path.

- [x] **Step 2: Update existing bless tests**

Adjust expectations where tests assumed `main` was mutated by default. Use default branch adoption where the behavior under test is adoption itself, and use `--direct` only where current-branch mutation is the behavior being protected.

- [x] **Step 3: Update docs**

Document that default `bless` creates a human adoption branch and that `--direct` is the opt-in direct-adoption mode.

- [x] **Step 4: Verify**

Run `just fmt`, `just test`, and `just lint`.
