# Doctor Repair Moved Checkout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd doctor --repair` support for the safe moved-human-checkout drift that `agd doctor` already detects.

**Architecture:** Extend the existing `doctor` command with a `--repair` flag. Repair only runs from a human checkout, resolves the current checkout root from Git, updates persisted project metadata when `human_checkout` differs, and repoints managed workspace `origin` fetch URLs at the repaired human checkout while leaving push denial intact.

**Tech Stack:** Rust, clap, system Git wrapper, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Repair Moved Human Checkout

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [ ] **Step 1: Write the failing test**

Add `doctor_repair_fixes_moved_human_checkout_path` to `tests/p0a.rs`. Initialize AGD, capture the workspace path, move the human checkout directory, run `agd doctor --repair` from the moved checkout, then assert:

- stdout contains `Repaired human checkout path`
- `agd doctor` succeeds from the moved checkout
- project metadata `human_checkout` equals the moved checkout path
- workspace `remote get-url origin` equals the moved checkout path
- workspace `remote get-url --push origin` remains `agd-deny://push-disabled`

- [ ] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_repair_fixes_moved_human_checkout_path`

Expected: FAIL because `doctor` currently has no `--repair` flag.

- [ ] **Step 3: Implement minimal repair**

Add `Doctor { repair: bool }` to `src/cli.rs`. In `src/main.rs`, route `agd doctor --repair` to `doctor::repair(&paths, &context, &cwd)`.

In `src/doctor.rs`, implement:

- require `ProjectContext::HumanCheckout`
- resolve current checkout with `git rev-parse --show-toplevel`
- clone and update `project.human_checkout`
- run `git remote set-url origin <current checkout>` for each workspace path that exists
- call `project::save_project`
- print repair messages

- [ ] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test doctor_repair_fixes_moved_human_checkout_path`

Expected: PASS.

- [ ] **Step 5: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/plans/2026-05-09-agd-doctor-repair-moved-checkout.md src/cli.rs src/main.rs src/doctor.rs tests/p0a.rs
git commit -m "feat: repair moved human checkout metadata"
```
