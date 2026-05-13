# AGD Sync Rebase Current Branch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `agd sync --rebase` infer the current agent branch when run from an agent workspace.

**Architecture:** Keep the existing explicit `agd sync --rebase <branch>` behavior. Extend CLI parsing so `--rebase` may be passed without a value, resolve that case in `main` using project discovery and the current checkout branch, and only allow inference from an agent workspace on an `agent/*` branch.

**Tech Stack:** Rust CLI with clap, existing sync module, integration tests in `tests/p0a.rs`.

---

### Task 1: Current Branch Rebase Inference

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `README.md`

- [x] **Step 1: Write failing coverage**

Add tests for `agd sync --rebase` from an agent workspace on the current agent branch, plus a guardrail test that bare `--rebase` from the human checkout fails with clear guidance.

- [x] **Step 2: Run the tests to verify failure**

Run `just dev cargo test sync_rebase_defaults_to_current_agent_branch_from_workspace` and confirm clap rejects the missing value.

- [x] **Step 3: Implement CLI and branch inference**

Allow optional values for `--rebase`, infer the current branch only from agent workspace mode, validate it with the existing branch policy, and preserve explicit branch behavior.

- [x] **Step 4: Verify**

Run the focused sync tests, `just fmt`, `just test`, and `just lint`.
