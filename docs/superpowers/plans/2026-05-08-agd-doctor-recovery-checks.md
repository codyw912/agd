# AGD Doctor Recovery Checks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `agd doctor` so it detects incomplete Git operations and existing AGD operation locks.

**Architecture:** Keep `doctor` as a read-only diagnostic command. Add checks for Git state files through `git rev-parse --git-path` so normal repos and future nonstandard Git dirs are handled consistently. Add an AGD lock check against `~/.agd/projects/<project-id>/locks/bless.lock`; report it without deleting it.

**Tech Stack:** Rust, system `git`, existing real-Git integration tests in `tests/p0a.rs`, `serde_json` for existing doctor JSON output.

---

## File Structure

- `src/doctor.rs`: add Git state and AGD lock checks.
- `tests/p0a.rs`: add integration tests for incomplete Git state and AGD lock detection.
- `docs/superpowers/plans/2026-05-08-agd-doctor-recovery-checks.md`: track this slice.

## Task 1: Incomplete Git State Test

- [x] Add `doctor_detects_incomplete_git_state` to `tests/p0a.rs`.
- [x] The test should:
  - initialize a human repo and AGD project;
  - create `.git/MERGE_HEAD` in the human checkout;
  - run `agd doctor` from the human checkout;
  - assert failure;
  - assert stdout contains `fail human git state`;
  - assert stdout contains `MERGE_HEAD`;
  - assert stderr contains `doctor found failed checks`.
- [x] Run `just dev cargo test doctor_detects_incomplete_git_state`.
- [x] Expected result: fail because doctor does not currently check incomplete Git operation state.

## Task 2: AGD Lock Test

- [x] Add `doctor_detects_existing_agd_operation_lock` to `tests/p0a.rs`.
- [x] The test should:
  - initialize a human repo and AGD project;
  - read `.git/agd/project.json` to get the project ID;
  - create `$AGD_HOME/projects/<project-id>/locks/bless.lock`;
  - run `agd doctor`;
  - assert failure;
  - assert stdout contains `fail AGD operation lock`;
  - assert stdout contains `bless.lock`;
  - assert stderr contains `doctor found failed checks`.
- [x] Run `just dev cargo test doctor_detects_existing_agd_operation_lock`.
- [x] Expected result: fail because doctor does not currently check AGD operation locks.

## Task 3: Doctor Implementation

- [x] Modify `src/doctor.rs`:
  - add a `git_state_check(name, repo)` helper;
  - check `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REBASE_HEAD`, `rebase-merge`, `rebase-apply`, and `index.lock`;
  - add checks for both human checkout and default workspace Git state;
  - add an `agd_lock_check(paths, project)` helper for `locks/bless.lock`;
  - append these checks to the existing doctor check list.
- [x] The Git state check should fail when any state path exists, with detail listing the state file or directory names.
- [x] The AGD lock check should pass when the lock is absent and fail when present.

## Task 4: Verification

- [x] Rerun `just dev cargo test doctor_detects_incomplete_git_state`.
- [x] Rerun `just dev cargo test doctor_detects_existing_agd_operation_lock`.
- [x] Run `just fmt`.
- [x] Run `just dev cargo fmt --all -- --check`.
- [x] Run `just test`.
- [x] Run `just lint`.
- [x] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: This implements PRD doctor detection for incomplete merge/cherry-pick/rebase state, stale Git lock files, and incomplete AGD operation locks. It intentionally does not implement `doctor --repair`, lock deletion, or `bless --continue/--abort`.

Placeholder scan: No TBD/TODO placeholders remain.

Type consistency: Check names are `human git state`, `workspace git state`, and `AGD operation lock`, matching the planned test assertions.
