# Bless Continue Conflict Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd bless --continue` for completing a conflicted squash adoption after the human resolves and stages conflicts.

**Architecture:** Persist a small AGD bless state file in the human checkout Git metadata before running squash adoption. The state records the adoption mode, branch, workspace, base, and tip needed to reconstruct the final signed commit message. On successful squash adoption, remove the state. On conflict, leave it in place. `agd bless --continue` reads the state, creates the signed adoption commit from the resolved index, removes the state, and prints completion. `agd bless --abort` should remove the state after resetting.

**Tech Stack:** Rust, serde JSON state, system Git, assert_cmd integration tests.

---

### Task 1: Continue Conflicted Squash Bless

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/adoption.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Write failing continue test**

Add an integration test that creates a squash conflict, resolves and stages the file, runs `agd bless --continue`, then asserts the human checkout is clean and the new signed commit contains AGD squash trailers.

- [x] **Step 2: Run focused test to verify failure**

Run: `just dev cargo test bless_continue_commits_resolved_squash_conflict`

Expected: FAIL because `--continue` is not currently accepted.

- [x] **Step 3: Add CLI shape and dispatch**

Add `--continue` to `agd bless`, make it mutually exclusive with `--abort` and branch adoption options, and dispatch to `adoption::continue_bless`.

- [x] **Step 4: Persist squash bless state**

Before running `git merge --squash`, write `.git/agd/bless.json` with the branch, workspace id, base, tip, and adoption mode. Remove it after successful commit. Leave it on conflict.

- [x] **Step 5: Implement continue**

Read the state, require squash adoption, commit the currently staged index with `git commit -S -m "Adopt <branch>" -m "<trailers>"`, remove the state, and print `Continued bless operation`.

- [x] **Step 6: Make abort clean persisted state**

After `git reset --merge`, remove any persisted bless state so abort leaves no recovery metadata behind.

- [x] **Step 7: Run focused test to verify pass**

Run: `just dev cargo test bless_continue_commits_resolved_squash_conflict`

Expected: PASS.

- [x] **Step 8: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.
