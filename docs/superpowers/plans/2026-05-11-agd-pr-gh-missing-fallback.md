# AGD PR Missing GH Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `agd pr` succeed with actionable next-step instructions when `gh` is not installed.

**Architecture:** Keep the existing fetch and push flow unchanged so the selected agent branch still reaches `origin`. Treat only an OS `NotFound` error from launching `gh` as a fallback; if `gh` exists and `gh pr create` fails, keep returning the existing failure.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Add Missing-GH Fallback

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`
- Modify: `src/main.rs`

- [x] **Step 1: Write the failing test**

Add a helper that builds a `PATH` containing `git` but not `gh`, then add `pr_falls_back_to_next_step_when_gh_is_missing`. The test creates an agent branch, runs `agd pr agent/pr-no-gh`, expects success with fallback instructions, and verifies `refs/heads/agent/pr-no-gh` exists in the bare origin.

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test pr_falls_back_to_next_step_when_gh_is_missing`

Expected: FAIL with `run gh` because `gh` is absent from `PATH`.

- [x] **Step 3: Write minimal implementation**

Change `PullRequestResult` to carry optional `url` and optional `next_step`. In `pull_request::open`, return `url: Some(...)` for successful `gh pr create`; when `Command::new("gh").output()` returns `ErrorKind::NotFound`, return `next_step: Some("Pushed ... Open a pull request ...")` after the push has completed. Update text output in `src/main.rs` to print the URL when present and otherwise print the fallback next step.

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
git add docs/superpowers/plans/2026-05-11-agd-pr-gh-missing-fallback.md src/main.rs src/pull_request.rs tests/p0a.rs
git commit -m "feat: fall back when gh is missing"
```
