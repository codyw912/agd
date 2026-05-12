# AGD PR Patch SHA Body Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Include `AGD-Patch-SHA256` in generated `agd pr` bodies so PR review metadata includes the same patch checksum used by adoption verification.

**Architecture:** Reuse the existing stable patch hash implementation in `src/provenance.rs`. `pull_request::pr_body` already reads the agent base and tip; compute the hash from those values in the agent workspace and append it to the existing `## Provenance` section.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Add Patch Checksum To PR Body

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write the failing test**

Extend `pr_command_pushes_agent_branch_and_invokes_gh` to compute the stable patch hash from `main` to `agent/pr-test` and assert the captured PR body contains `AGD-Patch-SHA256: <hash>`.

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test pr_command_pushes_agent_branch_and_invokes_gh`

Expected: FAIL because the PR body does not currently include `AGD-Patch-SHA256`.

- [x] **Step 3: Write minimal implementation**

Import `crate::provenance` in `src/pull_request.rs`, compute `provenance::patch_sha256(&workspace.path, base_commit.trim(), agent_tip.trim())?`, and append it to the existing Provenance section.

- [x] **Step 4: Run focused verification**

Run:

```bash
just dev cargo test pr_command_pushes_agent_branch_and_invokes_gh
just dev cargo test pr_
just dev cargo fmt --all -- --check
```

Expected: all PR-focused tests pass and formatting is clean.

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
git add docs/superpowers/plans/2026-05-12-agd-pr-patch-sha-body.md src/pull_request.rs tests/p0a.rs
git commit -m "feat: include PR patch checksum"
```
