# AGD JSON Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add machine-readable JSON output for the AGD commands most likely to be consumed by tools.

**Architecture:** Add a global `--json` flag and route selected commands through typed serializable response structs. Keep human output unchanged by default; JSON output is opt-in and stable enough for later TUI/IDE/harness integration.

**Tech Stack:** Rust, `clap`, `serde`, `serde_json`, system `git`, existing real-Git integration tests in `tests/p0a.rs`.

---

## File Structure

- `src/cli.rs`: add global `json: bool`.
- `src/main.rs`: branch selected command output on `cli.json`.
- `src/json_output.rs`: response structs and JSON rendering helpers.
- Existing modules may expose data helpers where currently they only print text.
- `tests/p0a.rs`: add integration tests that parse JSON for `path`, `status`, `branches`, `files`, and `doctor`.

## Task 1: Path and Status JSON

- [x] Add failing tests for `agd --json path` and `agd --json status`.
- [x] Run `just dev cargo test json_path_and_status_outputs_are_machine_readable`.
- [x] Implement global `--json`, JSON structs, and JSON rendering for path/status.
- [x] Rerun the test and confirm it passes.

## Task 2: Branches and Files JSON

- [x] Add a failing test that creates an agent branch and asserts `agd --json branches` returns a branch list and `agd --json files <branch>` returns changed files.
- [x] Run `just dev cargo test json_branches_and_files_outputs_are_machine_readable`.
- [x] Refactor review helpers to return data for JSON while preserving text output.
- [x] Rerun the test and confirm it passes.

## Task 3: Doctor JSON

- [x] Add a failing test that breaks a guardrail and asserts `agd --json doctor` returns check objects and exits nonzero.
- [x] Run `just dev cargo test json_doctor_outputs_checks`.
- [x] Expose doctor checks as serializable data and render JSON before returning failure.
- [x] Rerun the test and confirm it passes.

## Task 4: Verification

- [x] Run `just fmt`.
- [x] Run `just dev cargo fmt --all -- --check`.
- [x] Run `just test`.
- [x] Run `just lint`.
- [x] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: Covers the highest-value JSON interface commands from the PRD: `path`, `status`, `branches`, `files`, and `doctor`. JSON for `log` and `diff` remains deferred because their current raw Git text output is already useful and those schemas deserve a separate design.

Placeholder scan: No placeholder implementation steps remain.

Type consistency: Uses existing command names and project/workspace concepts.
