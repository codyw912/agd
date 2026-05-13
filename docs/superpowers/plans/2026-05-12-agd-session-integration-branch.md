# AGD Session Integration Branch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish `agent/main` as the default agent-local integration branch for long-running unattended sessions.

**Architecture:** Keep session management as normal Git for now. Update adoption branch derivation so `agent/main` blesses onto `adopt/main` by default instead of trying to create or mutate `main`, then document the recommended long-running session branch layout.

**Tech Stack:** Rust CLI with existing adoption helpers, README/PRD docs, integration tests in `tests/p0a.rs`.

---

### Task 1: Safe `agent/main` Adoption

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/adoption.rs`

- [x] **Step 1: Write the failing test**

Add an integration test proving `agd bless agent/main` creates/switches to `adopt/main` and leaves the human `main` branch unchanged.

- [x] **Step 2: Run the test to verify it fails**

Run: `just dev cargo test bless_agent_main_defaults_to_adopt_main_branch`

Expected: FAIL because current branch derivation strips `agent/` and attempts to use `main`.

- [x] **Step 3: Implement the special-case branch derivation**

Update `derive_adoption_branch` so `agent/main` maps to `adopt/main`, while ordinary agent branches continue to strip the `agent/` prefix.

- [x] **Step 4: Run the test to verify it passes**

Run: `just dev cargo test bless_agent_main_defaults_to_adopt_main_branch`

Expected: PASS.

### Task 2: Document Autonomous Session Flow

**Files:**
- Modify: `README.md`
- Modify: `PRD.md`

- [x] **Step 1: Update README**

Document `agent/main` as an agent-local integration branch and show feature branches merging into it during unattended work.

- [x] **Step 2: Update PRD**

Clarify that `agent/main` is a local integration branch, not the upstream target branch, and that `agd bless agent/main` adopts to `adopt/main` by default.

- [x] **Step 3: Verify**

Run `just fmt`, `just test`, and `just lint`.
