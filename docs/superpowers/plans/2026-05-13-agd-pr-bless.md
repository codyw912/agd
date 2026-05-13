# AGD PR Bless Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd pr --bless <agent-branch>` so protected-branch projects can create a human-owned adoption branch and PR in one command.

**Architecture:** Keep existing `agd pr <agent-branch>` behavior as the raw agent branch review path. Add a `--bless` flag that runs the existing squash adoption flow first, then pushes the resulting human adoption branch and opens a PR from that branch into the configured target. Reuse existing PR provider selection and fallback behavior.

**Tech Stack:** Rust CLI with clap, existing adoption and pull request modules, integration tests in `tests/p0a.rs`.

---

### Task 1: Bless-Backed PR

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write the failing test**

Add `pr_bless_creates_human_adoption_branch_and_invokes_gh`. The test should create a bare remote, initialize AGD, create `agent/pr-bless`, run `agd pr --bless agent/pr-bless`, and assert:
- the remote has `refs/heads/pr-bless`;
- the remote does not have `refs/heads/agent/pr-bless`;
- `gh pr create` used `--head pr-bless`;
- the human checkout is on `pr-bless`;
- the PR body includes the original agent branch provenance.

- [x] **Step 2: Run the test to verify it fails**

Run: `just dev cargo test pr_bless_creates_human_adoption_branch_and_invokes_gh`

Expected: FAIL because `agd pr` does not accept `--bless`.

- [x] **Step 3: Implement `pr --bless`**

Add the CLI flag, route it through a new pull request helper that performs squash adoption, pushes the adoption branch, and opens a PR from that branch.

- [x] **Step 4: Run the test to verify it passes**

Run: `just dev cargo test pr_bless_creates_human_adoption_branch_and_invokes_gh`

Expected: PASS.

- [x] **Step 5: Verify**

Run `just fmt`, `just test`, and `just lint`.
