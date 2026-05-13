# AGD Raw PR Body Reviewability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make raw `agd pr` descriptions read like normal reviewable PRs while preserving AGD provenance.

**Architecture:** Keep the existing raw PR publish flow and provenance fields. Rework only the generated body layout so reviewers see a human-oriented summary, commits, changed files, and adoption recommendation before the trailing provenance section.

**Tech Stack:** Rust CLI, existing pull request helper, integration tests in `tests/p0a.rs`.

---

### Task 1: Reviewable Raw PR Body

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write failing coverage**

Tighten the raw `agd pr` test to require a human-readable summary and review sections before provenance.

- [x] **Step 2: Run the test to verify failure**

Run `just dev cargo test pr_command_pushes_agent_branch_and_invokes_gh` and confirm the current body fails the new expectations.

- [x] **Step 3: Implement body layout**

Update raw PR body generation to lead with review context, preserve commits/files, move metadata into a Review Details section, and keep Provenance last.

- [x] **Step 4: Verify**

Run focused PR tests, `just fmt`, `just test`, and `just lint`.
