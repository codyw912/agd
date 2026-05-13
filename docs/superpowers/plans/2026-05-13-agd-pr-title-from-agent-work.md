# AGD PR Title From Agent Work Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make generated PR titles read like normal PR titles instead of branch names when AGD can infer a clear title.

**Architecture:** Keep branch names as the safe fallback. When the agent branch contains exactly one unique commit, use that commit subject as the hosted PR title for raw PRs, blessed PRs, and continued blessed PRs.

**Tech Stack:** Rust CLI, existing pull request helper, integration tests in `tests/p0a.rs`.

---

### Task 1: Commit-Derived PR Titles

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write failing coverage**

Update raw, blessed, and continued PR tests to expect the single agent commit subject as `--title`.

- [x] **Step 2: Run focused tests to verify failure**

Run the focused PR title tests and confirm the current implementation still uses branch names.

- [x] **Step 3: Implement title derivation**

Thread a separate title into GitHub/GitLab PR creation and derive it from the agent commit range when exactly one commit exists.

- [x] **Step 4: Verify**

Run focused PR tests, `just fmt`, `just test`, and `just lint`.
