# AGD Doctor Moved Checkout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `agd doctor` so it clearly detects when the human checkout was moved after `agd init`.

**Architecture:** Pass the discovery context and current directory into doctor reporting. When doctor is run from the human checkout, compare the current Git root with the stored `project.human_checkout` path using canonical paths where possible. Also add an explicit stored human-checkout existence check so moved or deleted paths are reported directly.

**Tech Stack:** Rust, system `git`, existing real-Git integration tests in `tests/p0a.rs`, existing `ProjectContext` discovery model.

---

## File Structure

- `src/doctor.rs`: accept `ProjectContext` and `cwd`, add human checkout existence/path checks.
- `src/main.rs`: pass the full project context and cwd to doctor text/JSON reporting.
- `tests/p0a.rs`: add integration coverage for a moved human checkout.
- `docs/superpowers/plans/2026-05-08-agd-doctor-moved-checkout.md`: track this slice.

## Task 1: Moved Checkout Test

- [x] Add `doctor_detects_moved_human_checkout_path` to `tests/p0a.rs`.
- [x] The test should:
  - initialize a human repo and AGD project;
  - rename the human checkout directory after `agd init`;
  - run `agd doctor` from the moved checkout path;
  - assert failure;
  - assert stdout contains `fail human checkout exists`;
  - assert stdout contains `fail human checkout path`;
  - assert stdout contains `current checkout`;
  - assert stderr contains `doctor found failed checks`.
- [x] Run `just dev cargo test doctor_detects_moved_human_checkout_path`.
- [x] Expected result: fail because doctor does not currently compare discovered checkout path to stored metadata.

## Task 2: Doctor Context Wiring

- [x] Modify `src/doctor.rs`:
  - change `doctor(paths, project)` to `doctor(paths, context, cwd)`;
  - change `report(paths, project)` to `report(paths, context, cwd)`;
  - derive `let project = context.project()` inside these functions.
- [x] Modify `src/main.rs`:
  - pass `&context` and `&cwd` to `doctor::doctor`;
  - pass `&context` and `&cwd` to `doctor::report` for JSON output.

## Task 3: Path Drift Checks

- [x] Modify `src/doctor.rs`:
  - add `path_check("human checkout exists", &project.human_checkout)` near the start of checks;
  - add `human_checkout_path_check(context, cwd)`;
  - when context is `ProjectContext::HumanCheckout`, compare `git rev-parse --show-toplevel` from `cwd` with `project.human_checkout`;
  - if paths differ, fail `human checkout path` with detail `metadata <path>, current checkout <path>`;
  - when context is `ProjectContext::AgentWorkspace`, return ok for `human checkout path` because the current Git root is the agent workspace.
- [x] Use canonicalized paths for comparison when both paths exist; fall back to direct path comparison if either path cannot be canonicalized.

## Task 4: Verification

- [x] Rerun `just dev cargo test doctor_detects_moved_human_checkout_path`.
- [x] Run `just fmt`.
- [x] Run `just dev cargo fmt --all -- --check`.
- [x] Run `just test`.
- [x] Run `just lint`.
- [x] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: This implements PRD doctor detection for moved human checkout paths. It intentionally does not implement `doctor --repair`, metadata rewriting, or agent origin URL repair.

Placeholder scan: No TBD/TODO placeholders remain.

Type consistency: Check names are `human checkout exists` and `human checkout path`, matching planned test assertions.
