# AGD Branches Summary Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd branches` useful as a quick review queue by showing commit and changed-file counts for each pending agent branch.

**Architecture:** Reuse the existing agent branch summary data already used by `agd status` and JSON status output. Keep `agd --json branches` unchanged so machine-readable callers are not forced into a schema migration.

**Tech Stack:** Rust CLI, existing review/status helpers, integration tests in `tests/p0a.rs`.

---

### Task 1: Branch Summary Text Output

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/review.rs`

- [x] **Step 1: Write failing coverage**

Update the `agd branches` integration coverage to expect branch name, commit count, and changed-file count in text output.

- [x] **Step 2: Run focused test to verify failure**

Run the focused branches test and confirm the current output only prints branch names.

- [x] **Step 3: Implement text summary output**

Have `agd branches` render branch summaries using the existing status summary source while leaving JSON branch output unchanged.

- [x] **Step 4: Verify**

Run the focused branches tests, `just fmt`, `just test`, and `just lint`.
