# AGD PR Continue Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `agd pr --continue` finish an interrupted `agd pr --bless` operation after the human adoption commit is recoverable.

**Architecture:** Persist target and adoption branch metadata in the bless recovery state. Add `agd pr --continue`, valid only without branch options, that runs the existing squash adoption continuation, then pushes the human adoption branch and opens the hosted PR using the stored base branch.

**Tech Stack:** Rust CLI with clap, existing adoption and pull request helpers, integration tests in `tests/p0a.rs`.

---

### Task 1: Blessed PR Continue

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/adoption.rs`
- Modify: `src/pull_request.rs`
- Modify: `README.md`

- [x] **Step 1: Write failing coverage**

Add a test that starts `agd pr --bless --target release --branch cody/pr-continue agent/pr-continue`, forces the signing commit to fail, then runs `agd pr --continue` and asserts the branch is pushed and the PR targets `release`.

- [x] **Step 2: Run the test to verify failure**

Run `just dev cargo test pr_continue_finishes_interrupted_blessed_pr` and confirm `agd pr` does not accept `--continue`.

- [x] **Step 3: Implement recovery metadata and command routing**

Persist target/adoption branch in bless state, return it from `bless --continue`, and add PR continuation that pushes and opens the PR.

- [x] **Step 4: Verify**

Run the focused PR recovery tests, `just fmt`, `just test`, and `just lint`.
