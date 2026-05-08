# AGD Signed Merge Bless Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd bless <branch> --merge` so a human can adopt an agent branch with a signed merge commit while leaving agent commits unchanged.

**Architecture:** Extend the existing `Bless` CLI command with a merge flag and add `AdoptionMode::Merge` beside squash and preserve. Reuse the current adoption preparation flow for clean checks, safety refs, lock acquisition, branch fetch, and base/tip metadata. Implement merge adoption with `git merge --no-ff -S <fetched-ref>` and AGD provenance trailers on the merge commit.

**Tech Stack:** Rust, `clap`, system `git`, existing real-Git integration tests in `tests/p0a.rs`.

---

## File Structure

- `src/cli.rs`: add `--merge` to `Command::Bless`.
- `src/main.rs`: select `AdoptionMode::Merge` and reject combining `--preserve` and `--merge`.
- `src/adoption.rs`: add `AdoptionMode::Merge` and signed merge implementation.
- `tests/p0a.rs`: add a real Git integration test for signed merge adoption.
- `docs/superpowers/plans/2026-05-08-agd-signed-merge-bless.md`: track this slice.

## Task 1: Signed Merge Test

- [x] Add `bless_merge_creates_signed_merge_commit_and_preserves_agent_commits` to `tests/p0a.rs`.
- [x] The test should:
  - initialize a human repo with `configure_fake_human_signer()`;
  - create two commits on `agent/refactor-auth` in the agent workspace;
  - run `agd bless agent/refactor-auth --merge` from the human checkout;
  - assert the human checkout has four commits total: initial, two agent commits, and one merge commit;
  - assert `git rev-list --parents -1 HEAD` has two parents;
  - assert the merge commit raw object includes a `gpgsig` header;
  - assert the merge commit body includes `AGD-Adoption: merge` and `AGD-Agent-Branch: agent/refactor-auth`;
  - assert the two agent commits remain authored and committed by `Local Agent <agent@agd.invalid>`.
- [x] Run `just dev cargo test bless_merge_creates_signed_merge_commit_and_preserves_agent_commits`.
- [x] Expected result: fail because `--merge` is not accepted yet.

## Task 2: CLI and Dispatch

- [x] Modify `src/cli.rs`:
  - add `#[arg(long)] merge: bool` to `Command::Bless`.
- [x] Modify `src/main.rs`:
  - reject `--preserve --merge` with `anyhow::bail!("choose only one bless adoption mode")`;
  - route `--merge` to `AdoptionMode::Merge`;
  - keep `--preserve` routed to `AdoptionMode::Preserve`;
  - keep the default routed to `AdoptionMode::Squash`.

## Task 3: Signed Merge Implementation

- [x] Modify `src/adoption.rs`:
  - add `Merge` to `AdoptionMode`;
  - dispatch `AdoptionMode::Merge` to a new `bless_merge` helper;
  - add a local `trailers(project, prepared, adoption)` helper to share the common AGD trailer block across squash and merge;
  - implement `bless_merge` with `git merge --no-ff -S <fetched-ref> -m "Merge <branch>" -m "<trailers>"`;
  - print `Merged <branch>`.
- [x] Preserve and squash output/behavior should remain unchanged.

## Task 4: Verification

- [x] Rerun `just dev cargo test bless_merge_creates_signed_merge_commit_and_preserves_agent_commits`.
- [x] Run `just fmt`.
- [x] Run `just dev cargo fmt --all -- --check`.
- [x] Run `just test`.
- [x] Run `just lint`.
- [x] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: This slice covers PRD signed merge mode and acceptance criterion 33. It intentionally does not implement policy checks for unsigned agent commits, `--force`, conflict continue/abort, or protected target policy configuration.

Placeholder scan: No TBD/TODO placeholders remain.

Type consistency: The plan uses `AdoptionMode::Merge` consistently across CLI dispatch and adoption implementation.
