# AGD Continue No-State Guidance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make continuation commands fail with a clear message when there is no pending recovery operation.

**Architecture:** Keep the existing bless and PR recovery state model. Improve the bless-state reader so missing `.git/agd/bless.json` becomes `no pending bless operation` instead of a raw file read error. `agd pr --continue` already checks PR publication state first, so when neither PR nor bless state exists it will inherit the same clear message.

**Tech Stack:** Rust CLI, existing adoption and PR modules, real-Git integration tests in `tests/p0a.rs`.

---

### Task 1: Missing Continuation State Guidance

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/adoption.rs`

- [x] **Step 1: Write failing bless coverage**

Add an integration test that runs `agd bless --continue` without pending bless state and expects `no pending bless operation`.

- [x] **Step 2: Write failing PR coverage**

Add an integration test that runs `agd pr --continue` without pending PR or bless state and expects the same clear guidance.

- [x] **Step 3: Implement missing-state message**

Handle `NotFound` when reading bless state and bail with `no pending bless operation`.

- [x] **Step 4: Verify**

Run focused continuation tests, `just fmt`, `just test`, and `just lint`.
