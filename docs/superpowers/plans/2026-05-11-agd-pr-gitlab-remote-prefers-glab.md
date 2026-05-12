# AGD PR GitLab Remote Prefers GLab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prefer `glab` over `gh` when `origin` is a GitLab remote.

**Architecture:** Keep the existing push flow and CLI fallback behavior. Add a small remote-provider check based on `remote.origin.url`; if it contains `gitlab`, try `glab mr create` before `gh pr create`, otherwise keep the current GitHub-first order.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Prefer GLab For GitLab Remotes

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write the failing test**

Add `pr_uses_glab_for_gitlab_origin_even_when_gh_exists`. Configure `origin.url` as `git@gitlab.com:example/project.git` and `origin.pushurl` as the local bare repo. Put fake `gh`, fake `glab`, and `git` on `PATH`. Run `agd pr agent/pr-gitlab`, expect the fake GitLab URL, verify the branch was pushed, and assert the `gh` capture file was not created.

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test pr_uses_glab_for_gitlab_origin_even_when_gh_exists`

Expected: FAIL because current PR creation tries `gh` first whenever `gh` exists.

- [x] **Step 3: Write minimal implementation**

Add a helper that reads `git config --get remote.origin.url` from the human checkout and treats URLs containing `gitlab` as GitLab remotes. For GitLab remotes, call `glab` first and fall back to manual instructions if it is missing; for non-GitLab remotes, keep the current `gh`, then `glab`, then manual order.

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
git add docs/superpowers/plans/2026-05-11-agd-pr-gitlab-remote-prefers-glab.md src/pull_request.rs tests/p0a.rs
git commit -m "feat: prefer glab for GitLab remotes"
```
