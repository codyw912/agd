# AGD Doctor PR State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor` detect an interrupted blessed PR publication and point users at `agd pr --continue`.

**Architecture:** Add a read-only doctor check for `.git/agd/pr.json`, using `git rev-parse --git-path` like the existing bless-state check so alternate Git directories keep working. This is diagnostic only: `doctor --repair` should not delete PR publication state because the adoption commit already exists and publication is recoverable.

**Tech Stack:** Rust CLI, existing doctor check model, real-Git integration tests in `tests/p0a.rs`.

---

### Task 1: Doctor PR Publication State

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/doctor.rs`

- [x] **Step 1: Write failing coverage**

Add an integration test that leaves `.git/agd/pr.json` in the human checkout and expects `agd doctor` to fail with `AGD PR state` and `agd pr --continue` guidance.

- [x] **Step 2: Add doctor check**

Implement an `agd_pr_state_check` next to the existing bless-state check and include it in the doctor report.

- [x] **Step 3: Verify**

Run focused doctor tests, `just fmt`, `just test`, and `just lint`.
