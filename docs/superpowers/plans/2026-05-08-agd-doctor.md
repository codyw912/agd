# AGD Doctor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd doctor` to inspect whether a project and its default agent workspace are configured safely.

**Architecture:** Reuse project discovery and the Git wrapper. Add a focused doctor module that evaluates independent checks and reports `ok`, `warn`, or `fail` lines, returning a nonzero exit code only for failed safety/configuration checks.

**Tech Stack:** Rust, `clap`, system `git`, existing real-Git integration tests in `tests/p0a.rs`.

---

## File Structure

- `src/cli.rs`: add `Doctor`.
- `src/main.rs`: dispatch doctor.
- `src/doctor.rs`: implement checks and report rendering.
- `tests/p0a.rs`: add integration tests for healthy workspaces, broken signing config, broken push guardrail, and missing workspace marker.

## Task 1: Healthy Doctor

- [ ] Add a failing test that runs `agd doctor` after `agd init` and expects success with healthy checks for metadata, workspace marker, agent identity, signing disabled, deny signer, and push denial.
- [ ] Run `just dev cargo test doctor_reports_healthy_workspace`.
- [ ] Implement `agd doctor` with those checks.
- [ ] Rerun the test and confirm it passes.

## Task 2: Broken Guardrails

- [ ] Add failing tests for `commit.gpgsign=true`, missing `gpg.program`, and push URL reset to the human checkout.
- [ ] Run `just dev cargo test doctor_reports_broken_guardrails`.
- [ ] Report failed checks and return nonzero.
- [ ] Rerun the test and confirm it passes.

## Task 3: Missing Workspace Marker

- [ ] Add a failing test that removes `.agd/workspace.json`, runs doctor from the human checkout, and expects a failed marker check.
- [ ] Run `just dev cargo test doctor_reports_missing_workspace_marker`.
- [ ] Add marker existence validation.
- [ ] Rerun the test and confirm it passes.

## Task 4: Verification

- [ ] Run `just fmt`.
- [ ] Run `just dev cargo fmt --all -- --check`.
- [ ] Run `just test`.
- [ ] Run `just lint`.
- [ ] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: Covers the first useful `agd doctor` checks: metadata discovery, workspace existence, marker, local identity, signing disabled, deny signer config, push URL denial, and dirty-state reporting as warnings. Repair, submodule/LFS, stale lock detection, and incomplete Git operation detection stay out of this slice.

Placeholder scan: No placeholder implementation steps remain.

Type consistency: Uses existing `Project`, default workspace metadata, `git::stdout`, and AGD path conventions.
