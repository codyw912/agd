# AGD Bless Current Branch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd bless` match AGD's review and PR ergonomics by defaulting to the current agent branch when run inside an agent workspace, while rejecting protected branches at the adoption boundary.

**Architecture:** Resolve an omitted `bless` branch in the CLI layer from the current branch only when project discovery says the command is running inside an agent workspace. Validate all branches passed to `adoption::bless` with the existing branch policy so direct adoption callers cannot bless protected mirror branches.

**Tech Stack:** Rust CLI, existing branch policy module, real-Git integration tests in `tests/p0a.rs`.

---

### Task 1: Current-Branch Bless Resolution

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/main.rs`
- Modify: `src/adoption.rs`

- [x] **Step 1: Write failing current-branch coverage**

Add an integration test that runs `agd bless` from inside an agent workspace on `agent/refactor-auth` and expects the branch to be blessed without passing a branch argument.

- [x] **Step 2: Implement branch inference**

Resolve omitted `bless` branches from the current agent workspace branch, and keep human-checkout invocations explicit.

- [x] **Step 3: Write failing protected-branch coverage**

Add integration coverage that `agd bless main` fails with `agent branch is required`.

- [x] **Step 4: Enforce adoption branch policy**

Validate branch names inside `adoption::bless` before any human checkout mutation.

- [x] **Step 5: Verify**

Run focused bless tests, `just fmt`, `just test`, and `just lint`.
