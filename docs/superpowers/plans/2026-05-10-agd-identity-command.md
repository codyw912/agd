# Identity Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd identity` as a read-only command showing the configured project agent identity.

**Architecture:** Reuse project discovery from either the human checkout or an agent workspace and print the existing `project.agent_identity` fields. Keep this slice display-only; future identity configuration remains separate.

**Tech Stack:** Rust, clap subcommand, existing project metadata, assert_cmd integration test.

---

### Task 1: Show Agent Identity

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Add: `src/identity.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Write failing identity test**

Add an integration test that runs `agd identity` after init from both the human checkout and default workspace, asserting it prints `Local Agent`, `agent@agd.invalid`, and `unsigned`.

- [x] **Step 2: Run focused test to verify failure**

Run: `just dev cargo test identity_shows_project_agent_identity`

Expected: FAIL because `agd identity` does not exist.

- [x] **Step 3: Add CLI and dispatch**

Add `Command::Identity` and route it through project discovery to an identity renderer.

- [x] **Step 4: Implement identity rendering**

Print name, email, and signing from `project.agent_identity`.

- [x] **Step 5: Run focused test to verify pass**

Run: `just dev cargo test identity_shows_project_agent_identity`

Expected: PASS.

- [x] **Step 6: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.
