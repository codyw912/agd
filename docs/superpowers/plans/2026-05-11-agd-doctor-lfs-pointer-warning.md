# AGD Doctor LFS Pointer Warning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor` warn when tracked Git LFS pointer files are present.

**Architecture:** Extend the existing `Git LFS` doctor check. Keep the current `.gitattributes` filter detection, and add a read-only `git grep` scan for the standard LFS pointer marker in tracked files from both the human checkout and default workspace.

**Tech Stack:** Rust CLI, system Git subprocesses, real-Git integration tests, `just` project commands.

---

### Task 1: Detect Tracked LFS Pointer Files

**Files:**
- Modify: `tests/p0a.rs`
- Modify: `src/doctor.rs`

- [x] **Step 1: Write the failing test**

Add `doctor_warns_when_lfs_pointer_files_are_tracked`. Commit `pointer.bin` with standard LFS pointer contents but no `.gitattributes`, run `agd doctor`, and assert:

```rust
.success()
.stdout(predicate::str::contains("warn Git LFS"))
.stdout(predicate::str::contains("pointer.bin"))
.stdout(predicate::str::contains("pointer"));
```

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_warns_when_lfs_pointer_files_are_tracked`

Expected: FAIL because the current check only detects `filter=lfs` declarations.

- [x] **Step 3: Write minimal implementation**

Add a helper that runs:

```bash
git grep -Il https://git-lfs.github.com/spec/v1 --
```

for the human checkout and workspace, then appends warning detail lines like `human pointer.bin looks like Git LFS pointer`.

- [x] **Step 4: Run focused verification**

Run:

```bash
just dev cargo test doctor_warns_when_lfs_pointer_files_are_tracked
just dev cargo test doctor_
just dev cargo fmt --all -- --check
```

Expected: all doctor-focused tests pass and formatting is clean.

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
git add docs/superpowers/plans/2026-05-11-agd-doctor-lfs-pointer-warning.md src/doctor.rs tests/p0a.rs
git commit -m "feat: warn on LFS pointer files"
```
