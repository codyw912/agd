# AGD Doctor Submodule/LFS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `agd doctor` so it reports submodule and Git LFS usage as warnings.

**Architecture:** Keep doctor read-only. Detect submodules by checking for `.gitmodules` in the stored human checkout and default workspace. Detect Git LFS usage by scanning `.gitattributes` files for `filter=lfs`; optionally include whether `git lfs version` is available in the warning detail. These checks warn rather than fail, matching the PRD’s P0 behavior.

**Tech Stack:** Rust, system `git`, existing real-Git integration tests in `tests/p0a.rs`.

---

## File Structure

- `src/doctor.rs`: add submodule and LFS warning checks.
- `tests/p0a.rs`: add integration tests for `.gitmodules` and LFS attributes.
- `docs/superpowers/plans/2026-05-08-agd-doctor-submodule-lfs.md`: track this slice.

## Task 1: Submodule Warning Test

- [x] Add `doctor_warns_when_submodules_are_declared` to `tests/p0a.rs`.
- [x] The test should:
  - initialize a human repo;
  - create and commit a `.gitmodules` file before `agd init`;
  - run `agd init`;
  - run `agd doctor`;
  - assert success;
  - assert stdout contains `warn submodules`;
  - assert stdout contains `.gitmodules`.
- [x] Run `just dev cargo test doctor_warns_when_submodules_are_declared`.
- [x] Expected result: fail because doctor does not currently check submodule declarations.

## Task 2: LFS Warning Test

- [x] Add `doctor_warns_when_lfs_filters_are_declared` to `tests/p0a.rs`.
- [x] The test should:
  - initialize a human repo;
  - create and commit `.gitattributes` containing `*.bin filter=lfs diff=lfs merge=lfs -text` before `agd init`;
  - run `agd init`;
  - run `agd doctor`;
  - assert success;
  - assert stdout contains `warn Git LFS`;
  - assert stdout contains `filter=lfs`.
- [x] Run `just dev cargo test doctor_warns_when_lfs_filters_are_declared`.
- [x] Expected result: fail because doctor does not currently check LFS filters.

## Task 3: Doctor Implementation

- [x] Modify `src/doctor.rs`:
  - add `submodule_check(project, workspace_path)`;
  - warn if either `project.human_checkout/.gitmodules` or `<workspace>/.gitmodules` exists;
  - add `lfs_check(project, workspace_path)`;
  - warn if `.gitattributes` in either checkout contains `filter=lfs`;
  - include `git-lfs available` or `git-lfs unavailable` in LFS warning detail using `git lfs version`.
- [x] Add these checks after workspace marker validation and before identity guardrail checks.
- [x] Warnings must not make `doctor` return failure.

## Task 4: Verification

- [x] Rerun `just dev cargo test doctor_warns_when_submodules_are_declared`.
- [x] Rerun `just dev cargo test doctor_warns_when_lfs_filters_are_declared`.
- [x] Run `just fmt`.
- [x] Run `just dev cargo fmt --all -- --check`.
- [x] Run `just test`.
- [x] Run `just lint`.
- [x] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: This implements PRD submodule and Git LFS detection as doctor warnings. It intentionally does not configure submodule modes, initialize submodules, fetch LFS content, or block workspace creation.

Placeholder scan: No TBD/TODO placeholders remain.

Type consistency: Check names are `submodules` and `Git LFS`, matching planned test assertions.
