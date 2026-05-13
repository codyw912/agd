# AGD PR Bless Target Options Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `agd pr --bless` use explicit adoption branch names and non-default target branches.

**Architecture:** Extend the `pr` CLI with `--target` and `--branch` options that are valid only with `--bless`. Thread the selected target into adoption and PR provider creation so the human-owned adoption branch is created from the requested base and the hosted PR targets the same base.

**Tech Stack:** Rust CLI with clap, existing adoption and pull request helpers, integration tests in `tests/p0a.rs`.

---

### Task 1: Blessed PR Branch And Target Options

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write failing coverage**

Add a test for `agd pr --bless --target release --branch cody/pr-bless agent/pr-bless`, asserting the remote branch is `cody/pr-bless`, `gh pr create` uses `--base release` and `--head cody/pr-bless`, and the human checkout is on `cody/pr-bless`.

- [x] **Step 2: Run the test to verify failure**

Run `just dev cargo test pr_bless_uses_explicit_target_and_adoption_branch`.

Expected: FAIL because `agd pr` does not yet accept `--target` or `--branch`.

- [x] **Step 3: Implement CLI and PR routing**

Add the CLI options, reject them without `--bless`, pass them into the blessed PR helper, and make PR provider creation accept an explicit base branch.

- [x] **Step 4: Verify**

Run `just dev cargo test pr_bless_uses_explicit_target_and_adoption_branch`, `just dev cargo test pr_`, `just fmt`, `just test`, and `just lint`.
