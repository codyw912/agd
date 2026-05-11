# AGD Doctor Workspace Missing Project Metadata Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor` report missing user-local project metadata when run from an agent workspace.

**Architecture:** Extend the doctor-only metadata fallback added for human checkout markers so it can also read an agent workspace `.agd/workspace.json` marker. Reuse the same failed `project metadata` report path and leave normal discovery behavior unchanged for non-doctor commands.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Report Missing Metadata From Agent Workspace

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/doctor.rs`

- [x] **Step 1: Write the failing test**

Add `doctor_reports_missing_project_metadata_from_agent_workspace` that initializes AGD, captures the workspace path, deletes the user-local `project.json`, then runs `agd doctor` from the agent workspace and expects:

```rust
.failure()
.stdout(predicate::str::contains("fail project metadata"))
.stdout(predicate::str::contains("missing"))
.stderr(predicate::str::contains("doctor found failed checks"));
```

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_reports_missing_project_metadata_from_agent_workspace`

Expected: FAIL because the current metadata fallback only checks the human checkout marker.

- [x] **Step 3: Write minimal implementation**

In `doctor::metadata_failure_report`, fall back to walking ancestors from `cwd` for `.agd/workspace.json`, parse its `project_id`, and call the existing project metadata failure detail helper with that project id.

- [x] **Step 4: Run focused verification**

Run:

```bash
just dev cargo test doctor_reports_missing_project_metadata_from_agent_workspace
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
git add docs/superpowers/plans/2026-05-11-agd-doctor-workspace-missing-project-metadata.md src/doctor.rs tests/p0a.rs
git commit -m "fix: report missing metadata from workspaces"
```
