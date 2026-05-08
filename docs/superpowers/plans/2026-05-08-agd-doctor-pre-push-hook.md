# AGD Doctor Pre-Push Hook Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `agd doctor` so it detects when the AGD-installed pre-push hook is missing or stale.

**Architecture:** Keep push URL denial as the primary enforcement. Add a read-only doctor check for `<workspace>/.git/hooks/pre-push` that verifies the hook exists and contains the AGD push-denial message. Missing or stale hooks should be a failed doctor check because AGD installs the hook and expects it as a UX guardrail.

**Tech Stack:** Rust, system `git`, existing real-Git integration tests in `tests/p0a.rs`.

---

## File Structure

- `src/doctor.rs`: add a pre-push hook check.
- `tests/p0a.rs`: add integration coverage for missing hook detection.
- `docs/superpowers/plans/2026-05-08-agd-doctor-pre-push-hook.md`: track this slice.

## Task 1: Missing Hook Test

- [x] Add `doctor_detects_missing_pre_push_hook` to `tests/p0a.rs`.
- [x] The test should:
  - initialize a human repo and AGD project;
  - remove `<workspace>/.git/hooks/pre-push`;
  - run `agd doctor`;
  - assert failure;
  - assert stdout contains `fail pre-push hook`;
  - assert stdout contains `missing`;
  - assert stderr contains `doctor found failed checks`.
- [x] Run `just dev cargo test doctor_detects_missing_pre_push_hook`.
- [x] Expected result: fail because doctor does not currently check the hook.

## Task 2: Doctor Implementation

- [x] Modify `src/doctor.rs`:
  - add `pre_push_hook_check(workspace_path)`;
  - check `<workspace>/.git/hooks/pre-push`;
  - fail with detail `missing <path>` if absent;
  - fail with detail `stale <path>` if the file does not contain `AGD: push is disabled for this agent workspace.`;
  - pass when the hook exists and contains the expected message.
- [x] Add the check after `push disabled` and before Git state checks.

## Task 3: Verification

- [x] Rerun `just dev cargo test doctor_detects_missing_pre_push_hook`.
- [x] Run `just fmt`.
- [x] Run `just dev cargo fmt --all -- --check`.
- [x] Run `just test`.
- [x] Run `just lint`.
- [x] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: This implements PRD doctor detection for the expected pre-push hook. It intentionally does not implement `doctor --repair` or hook rewriting.

Placeholder scan: No TBD/TODO placeholders remain.

Type consistency: Check name is `pre-push hook`, matching planned test assertions.
