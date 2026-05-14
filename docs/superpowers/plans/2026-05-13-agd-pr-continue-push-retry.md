# AGD PR Continue Push Retry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd pr --continue` retryable after a blessed PR adoption commit succeeds but the remote push or hosted PR creation fails.

**Architecture:** Persist a small PR recovery state in the human checkout Git metadata after blessed adoption succeeds and before remote publication begins. `agd pr --continue` first resumes any pending PR publication state; if none exists, it keeps the existing behavior of continuing an interrupted bless operation. Remove the PR state only after the remote branch is pushed and the hosted PR is created or a manual next step is printed.

**Tech Stack:** Rust CLI, system Git subprocesses, existing real-Git integration tests in `tests/p0a.rs`.

---

### Task 1: Retry Blessed PR Publication

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write failing retry coverage**

Extend PR continue coverage so a first `pr --continue` completes adoption but fails remote push, then a second `pr --continue` retries publication successfully.

- [x] **Step 2: Persist PR recovery state**

Write `.git/agd/pr.json` after blessed adoption succeeds and before pushing the human adoption branch.

- [x] **Step 3: Resume PR publication**

Have `agd pr --continue` consume pending PR state before attempting to continue bless state, and delete the PR state only after publication reaches a terminal successful result.

- [x] **Step 4: Verify**

Run focused PR continue tests, `just fmt`, `just test`, and `just lint`.
