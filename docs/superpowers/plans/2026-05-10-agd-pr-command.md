# PR Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a minimal `agd pr [branch]` command that lets the human checkout publish an agent branch and open a GitHub PR with useful AGD context.

**Architecture:** Keep the agent workspace without push credentials. From the human checkout, fetch the selected agent branch from the managed workspace into an AGD ref, push that ref to the human checkout's `origin` as the same branch name, then shell out to `gh pr create`. Build a concise PR body from existing Git review data: base branch, agent branch, commit list, changed files, and adoption recommendation. `glab` and non-GitHub fallback can remain later slices.

**Tech Stack:** Rust, system Git, GitHub CLI (`gh`) subprocess, assert_cmd integration test with a fake `gh`.

---

### Task 1: Minimal GitHub PR Command

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Add: `src/pull_request.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Write failing PR command test**

Add an integration test that initializes a local bare remote, creates an agent branch in the workspace, puts a fake `gh` executable on `PATH`, runs `agd pr agent/pr-test`, and asserts:
- the branch was pushed to the bare remote;
- `gh pr create` was invoked with base, head, title, and body;
- command stdout includes the fake PR URL.

- [x] **Step 2: Run focused test to verify failure**

Run: `just dev cargo test pr_command_pushes_agent_branch_and_invokes_gh`

Expected: FAIL because `agd pr` does not exist.

- [x] **Step 3: Add CLI and dispatch**

Add `Command::Pr { branch: Option<String> }` and route it through project discovery to `pull_request::open`.

- [x] **Step 4: Implement branch publish**

Resolve the selected branch like review commands do. Fetch `refs/heads/<branch>` from the managed workspace into `refs/agd/pr/<branch>`, then push that ref from the human checkout to `origin` as `refs/heads/<branch>`.

- [x] **Step 5: Generate PR body and invoke gh**

Collect commit subjects and changed files from the workspace. Invoke `gh pr create --base <default_target> --head <branch> --title "<branch>" --body "<body>"` from the human checkout and print its stdout.

- [x] **Step 6: Run focused test to verify pass**

Run: `just dev cargo test pr_command_pushes_agent_branch_and_invokes_gh`

Expected: PASS.

- [x] **Step 7: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.
