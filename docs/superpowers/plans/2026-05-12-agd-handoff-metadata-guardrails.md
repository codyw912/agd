# AGD Handoff Metadata Guardrails Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use TDD/tracer-bullet execution for this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd handoff` metadata reflect the full handoff and add regression coverage for unsafe selected untracked paths.

**Architecture:** Keep handoff behavior unchanged: clean agent workspace required, tracked human diff applied as uncommitted workspace changes, selected untracked files copied only when explicit, no auto-commit. Extend the persisted `.git/agd/handoff.json` with the same status, tracked file list, and untracked file list already returned by JSON output. Add tests for absolute and parent-directory selected paths to pin the existing path-safety contract.

**Tech Stack:** Rust CLI, system Git subprocesses, JSON metadata, real-Git integration tests, `just` project commands.

---

### Task 1: Enrich Handoff Metadata And Guard Path Selection

**Files:**
- Modify: `src/handoff.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Add failing metadata coverage**

Update `handoff_applies_human_tracked_changes_to_clean_agent_workspace` and `handoff_copies_selected_untracked_human_files` to parse `.git/agd/handoff.json` and assert it includes:

- `status: "applied"`
- `tracked_files`
- `untracked_files`
- existing project/workspace/head fields

- [x] **Step 2: Add unsafe path guard coverage**

Add `handoff_rejects_unsafe_selected_untracked_paths`, covering `../outside.txt` and an absolute path. Assert both fail with `untracked handoff path must be relative`.

- [x] **Step 3: Run tests to verify failure**

Run:

```bash
just dev cargo test handoff_applies_human_tracked_changes_to_clean_agent_workspace
just dev cargo test handoff_copies_selected_untracked_human_files
just dev cargo test handoff_rejects_unsafe_selected_untracked_paths
```

Expected: metadata assertions fail before implementation; unsafe path tests pass if existing guardrails already cover them.

- [x] **Step 4: Implement metadata enrichment**

Add `status` and `tracked_files` to `HandoffMetadata`, pass the already-computed tracked/untracked lists into `write_metadata`, and persist the same values that command output reports.

- [x] **Step 5: Run focused verification**

Run:

```bash
just dev cargo test handoff_
just dev cargo test json_handoff_outputs_applied_changes
just dev cargo fmt --all -- --check
```

Expected: handoff-focused tests pass and formatting is clean.

- [x] **Step 6: Run full verification**

Run:

```bash
just fmt
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all verification commands pass.

- [x] **Step 7: Commit**

```bash
git add docs/superpowers/plans/2026-05-12-agd-handoff-metadata-guardrails.md src/handoff.rs tests/p0a.rs
git commit -m "feat: enrich handoff metadata"
```
