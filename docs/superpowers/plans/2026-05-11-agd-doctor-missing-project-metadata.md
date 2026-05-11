# AGD Doctor Missing Project Metadata Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor` report missing user-local project metadata as a failed doctor check.

**Architecture:** Keep normal project discovery unchanged for regular commands. Add a doctor-only fallback path that recognizes a human checkout marker, inspects the referenced user-local `project.json`, and emits a minimal failed `project metadata` report when that metadata is missing or unreadable.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Report Missing Project Metadata

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/main.rs`
- Modify: `src/doctor.rs`

- [x] **Step 1: Write the failing test**

Add `doctor_reports_missing_project_metadata` that initializes AGD, deletes the user-local `project.json`, then runs `agd doctor` from the human checkout and expects:

```rust
.failure()
.stdout(predicate::str::contains("fail project metadata"))
.stdout(predicate::str::contains("missing"))
.stderr(predicate::str::contains("doctor found failed checks"));
```

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_reports_missing_project_metadata`

Expected: FAIL because `agd doctor` exits with a raw metadata read error instead of a doctor report.

- [x] **Step 3: Write minimal implementation**

Add a `doctor::metadata_failure_report(paths, cwd)` helper that returns `Some(DoctorReport)` when the current checkout has a human AGD marker but the referenced project metadata cannot be loaded. Route non-repair `agd doctor` and `agd --json doctor` through that fallback when normal discovery fails.

- [x] **Step 4: Run focused verification**

Run:

```bash
just dev cargo test doctor_reports_missing_project_metadata
just dev cargo test doctor_
just dev cargo fmt --all -- --check
```

Expected: all doctor-focused tests pass and formatting is clean.

- [x] **Step 5: Run full verification**

Run:

```bash
just fmt
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all verification commands pass.

- [x] **Step 6: Commit**

```bash
git add docs/superpowers/plans/2026-05-11-agd-doctor-missing-project-metadata.md src/main.rs src/doctor.rs tests/p0a.rs
git commit -m "fix: report missing project metadata in doctor"
```
