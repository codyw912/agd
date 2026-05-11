# AGD PR Provenance Trailers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add AGD-style provenance trailer lines to generated `agd pr` bodies.

**Architecture:** Reuse the existing base commit and agent tip lookups already performed by `src/pull_request.rs`. Append a dedicated `## Provenance` section to the PR body with `AGD-Agent-Branch`, `AGD-Agent-Base`, and `AGD-Agent-Tip` lines so reviewers have copyable machine-readable metadata without changing push or `gh` behavior.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Add PR Body Provenance Trailers

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write the failing test**

In `pr_command_pushes_agent_branch_and_invokes_gh`, assert the captured `gh pr create` body contains:

```rust
assert!(gh_args.contains("## Provenance"));
assert!(gh_args.contains("AGD-Agent-Branch: agent/pr-test"));
assert!(gh_args.contains(&format!("AGD-Agent-Base: {}", base_commit.trim())));
assert!(gh_args.contains(&format!("AGD-Agent-Tip: {}", agent_tip.trim())));
```

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test pr_command_pushes_agent_branch_and_invokes_gh`

Expected: FAIL because the PR body does not include an AGD provenance section.

- [x] **Step 3: Write minimal implementation**

In `pr_body`, append this section before adoption recommendation:

```text
## Provenance
AGD-Agent-Branch: {branch}
AGD-Agent-Base: <base commit>
AGD-Agent-Tip: <agent tip>
```

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
git add docs/superpowers/plans/2026-05-11-agd-pr-provenance-trailers.md src/pull_request.rs tests/p0a.rs
git commit -m "feat: include PR provenance trailers"
```
