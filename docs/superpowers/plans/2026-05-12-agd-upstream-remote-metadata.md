# AGD Upstream Remote Metadata Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the human checkout's original upstream remote in AGD metadata and use it for clearer PR fallback instructions.

**Architecture:** Add optional upstream remote metadata to `Project`. Capture `remote.origin.url` and `remote.origin.pushurl` during `agd init` without changing the human checkout. Surface the metadata in JSON init/status responses. When `agd pr` can push the branch but cannot launch `gh`/`glab`, derive a hosted GitHub/GitLab PR/MR URL from the stored upstream remote when possible; otherwise keep the existing generic next-step text.

**Tech Stack:** Rust CLI, system Git subprocesses, serde-compatible project metadata evolution, real-Git integration tests, `just` project commands.

---

### Task 1: Upstream Remote Metadata And PR Fallback URLs

**Files:**
- Modify: `src/project.rs`
- Modify: `src/json_output.rs`
- Modify: `src/pull_request.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Add failing upstream metadata test**

Add a test that initializes AGD after a human checkout has an `origin` fetch URL and a distinct push URL. Assert that `--json init` and the persisted project metadata include the original upstream remote fields.

- [x] **Step 2: Add failing PR fallback URL tests**

Add tests for GitHub and GitLab remotes with local push URLs. Run `agd pr` with only `git` on `PATH`, assert the branch is pushed, and assert the printed next step includes a hosted PR/MR creation URL.

- [x] **Step 3: Run tests to verify they fail**

Run:

```bash
just dev cargo test init_records_upstream_remote_metadata
just dev cargo test pr_fallback_includes_github_compare_url
just dev cargo test pr_fallback_includes_gitlab_mr_url
```

Expected: FAIL because upstream metadata and hosted fallback URLs are not implemented.

- [x] **Step 4: Implement upstream remote metadata**

Add optional serde-compatible project metadata for `origin`, capture fetch/push URLs during init, and include the metadata in JSON init/status responses.

- [x] **Step 5: Implement hosted fallback URL guidance**

Use stored upstream metadata to derive GitHub compare URLs and GitLab new merge request URLs. Keep the existing generic fallback when no hosted URL can be derived.

- [x] **Step 6: Run focused verification**

Run:

```bash
just dev cargo test init_records_upstream_remote_metadata
just dev cargo test pr_fallback_includes_github_compare_url
just dev cargo test pr_fallback_includes_gitlab_mr_url
just dev cargo test pr_falls_back_to_next_step_when_gh_is_missing
just dev cargo test pr_uses_glab_for_gitlab_origin_even_when_gh_exists
just dev cargo fmt --all -- --check
```

Expected: remote metadata and PR-focused tests pass and formatting is clean.

- [x] **Step 7: Run full verification**

Run:

```bash
just fmt
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all verification commands pass.

- [x] **Step 8: Commit**

```bash
git add docs/superpowers/plans/2026-05-12-agd-upstream-remote-metadata.md src/project.rs src/json_output.rs src/pull_request.rs tests/p0a.rs
git commit -m "feat: track upstream remote metadata"
```
