# AGD PR GLab Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Use `glab` to create a GitLab merge request when `gh` is not installed.

**Architecture:** Keep the existing fetch, push, PR body, `gh pr create`, and manual fallback behavior. If launching `gh` returns `NotFound`, try `glab mr create` before returning manual next-step instructions.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Add GLab MR Fallback

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write the failing test**

Add a fake `glab` helper and `pr_uses_glab_when_gh_is_missing`. The test should create an agent branch, run `agd pr`, assert stdout is the fake GitLab MR URL, verify the branch was pushed, and assert captured args contain:

```text
mr
create
--target-branch
main
--source-branch
agent/pr-glab
--title
agent/pr-glab
--description
```

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test pr_uses_glab_when_gh_is_missing`

Expected: FAIL because current missing-`gh` handling returns manual next-step instructions instead of invoking `glab`.

- [x] **Step 3: Write minimal implementation**

In `pull_request::open`, when `gh` is missing, run:

```bash
glab mr create --target-branch <default> --source-branch <branch> --title <branch> --description <body>
```

Return the trimmed stdout URL on success. If `glab` is also missing, preserve the current manual fallback. If `glab` runs and fails, return `glab mr create failed: ...`.

- [x] **Step 4: Run focused verification**

Run:

```bash
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
git add docs/superpowers/plans/2026-05-11-agd-pr-glab-fallback.md src/pull_request.rs tests/p0a.rs
git commit -m "feat: use glab when gh is missing"
```
