# AGD PR Agent Tip Body Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Include the agent branch tip commit SHA in the generated `agd pr` body.

**Architecture:** Extend the existing PR body construction in `src/pull_request.rs` with one additional Git lookup against the managed workspace. Keep the behavior scoped to presentation metadata; branch resolution, pushing, and `gh pr create` invocation stay unchanged.

**Tech Stack:** Rust CLI, real Git integration tests, `just` project commands.

---

### Task 1: Add Agent Tip Provenance

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/pull_request.rs`

- [x] **Step 1: Write the failing test**

Add this assertion setup to `pr_command_pushes_agent_branch_and_invokes_gh` after the agent commit:

```rust
let agent_tip = fixture.git_stdout(&workspace, ["rev-parse", "agent/pr-test"]);
```

Assert the captured `gh pr create` arguments include the tip:

```rust
assert!(gh_args.contains(&format!("Agent tip: {}", agent_tip.trim())));
```

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test pr_command_pushes_agent_branch_and_invokes_gh`

Expected: FAIL because the PR body does not include `Agent tip`.

- [x] **Step 3: Write minimal implementation**

In `pr_body`, resolve the branch tip from the managed workspace:

```rust
let agent_tip = git::stdout(&workspace.path, ["rev-parse", branch])?;
```

Add `- Agent tip: {}` to the Summary section, using `agent_tip.trim()`.

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
git add docs/superpowers/plans/2026-05-11-agd-pr-agent-tip-body.md src/pull_request.rs tests/p0a.rs
git commit -m "feat: include PR agent tip"
```
