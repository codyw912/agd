# Doctor Repair Guardrails Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor --repair` restore AGD workspace guardrails that `agd doctor` already detects as broken.

**Architecture:** Reuse the existing guardrail installer instead of duplicating config writes in doctor repair. When repair runs from the human checkout, iterate managed workspaces, repair the human checkout path/origin as before, then reinstall workspace identity, signing denial, push denial, and the pre-push hook.

**Tech Stack:** Rust, clap, system Git wrapper, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Repair Workspace Guardrails

**Files:**
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [ ] **Step 1: Write the failing test**

Add `doctor_repair_restores_workspace_guardrails` to `tests/p0a.rs`. Initialize AGD, break the workspace by setting `commit.gpgsign=true`, unsetting `gpg.program`, changing `remote.origin.pushurl` to the human checkout, and removing `.git/hooks/pre-push`. Run `agd doctor --repair`, then assert:

- stdout contains `Repaired workspace guardrails`
- `agd doctor` succeeds
- `commit.gpgsign` is `false`
- `gpg.program` points at an existing file
- push URL is `agd-deny://push-disabled`
- pre-push hook exists again

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_repair_restores_workspace_guardrails`

Expected: FAIL because current `doctor --repair` only repairs moved checkout metadata and workspace origin.

- [ ] **Step 3: Implement minimal repair**

In `src/doctor.rs`, import `crate::guardrails` and call `guardrails::install(paths, &workspace.path)?` for each existing workspace in `repair`. Print `Repaired workspace guardrails for <workspace-id>`.

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test doctor_repair_restores_workspace_guardrails`

Expected: PASS.

- [ ] **Step 5: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/plans/2026-05-09-agd-doctor-repair-guardrails.md src/doctor.rs tests/p0a.rs
git commit -m "feat: repair workspace guardrails"
```
