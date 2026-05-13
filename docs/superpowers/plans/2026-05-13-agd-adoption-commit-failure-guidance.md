# AGD Adoption Commit Failure Guidance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make adoption commit signing failures tell users exactly how to recover.

**Architecture:** Keep the existing bless recovery state and continuation commands. Wrap squash adoption commit failures with command-specific guidance: plain `agd bless` points to `agd bless --continue`, while `agd pr --bless` points to `agd pr --continue`.

**Tech Stack:** Rust CLI, existing adoption and pull request helpers, integration tests in `tests/p0a.rs`.

---

### Task 1: Recovery Guidance For Commit Failures

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/adoption.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write failing coverage**

Add tests for failed signing during plain `agd bless` and `agd pr --bless`, asserting the recovery guidance mentions the right continuation command.

- [x] **Step 2: Run the tests to verify failure**

Run the focused tests and confirm at least the PR path still lacks `agd pr --continue` guidance.

- [x] **Step 3: Implement guided errors**

Catch squash commit failures after merge state has been written, include bless continuation guidance, and wrap blessed PR adoption failures with PR continuation guidance.

- [x] **Step 4: Verify**

Run focused adoption/PR tests, `just fmt`, `just test`, and `just lint`.
