# AGD PR Provenance Link Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a concise AGD repository link to generated PR bodies so reviewers can understand AGD provenance fields.

**Architecture:** Keep the change scoped to PR body rendering. Add the same one-line AGD help link under the `## Provenance` section for both raw agent PRs and blessed adoption PRs, and pin both paths with integration test assertions.

**Tech Stack:** Rust CLI, existing PR body helpers in `src/pull_request.rs`, integration tests in `tests/p0a.rs`.

---

### Task 1: Provenance Help Link

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write failing test assertions**

Assert raw `agd pr` and `agd pr --bless` generated bodies contain `AGD provenance: https://github.com/codyw912/agd`.

- [x] **Step 2: Run focused tests to verify failure**

Run: `just dev cargo test pr_`

Expected: FAIL because generated PR bodies do not include the link yet.

- [x] **Step 3: Add the PR body link**

Add one helper string and include it in both PR body templates under `## Provenance`.

- [x] **Step 4: Verify**

Run `just dev cargo test pr_`, `just fmt`, `just test`, and `just lint`.
