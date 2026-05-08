# AGD Preserve Bless Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `agd bless <branch> --preserve` so a human can replay agent commits into the human checkout as signed commits while preserving each original agent author.

**Architecture:** Extend the `Bless` CLI command with a preserve flag and route adoption through an explicit mode enum. Keep the existing squash implementation behavior unchanged. Implement preserve by fetching the agent branch into a temporary AGD ref, enumerating commits unique to the target, applying each commit with `git cherry-pick --no-commit`, then committing with `git commit -S -C <agent-commit>` plus AGD trailers so author metadata is preserved and the human checkout's signer is invoked at adoption time.

**Tech Stack:** Rust, `clap`, system `git`, existing real-Git integration tests in `tests/p0a.rs`.

---

## File Structure

- `src/cli.rs`: add `--preserve` to `Command::Bless`.
- `src/main.rs`: map `Bless` args to an adoption mode.
- `src/adoption.rs`: add `AdoptionMode`, shared setup/fetch helpers, and preserve replay.
- `tests/p0a.rs`: add a real Git integration test for preserve replay.
- `docs/superpowers/plans/2026-05-08-agd-preserve-bless.md`: track this slice.

## Task 1: Preserve Bless Test

- [x] Add `bless_preserve_replays_agent_commits_with_human_signatures` to `tests/p0a.rs`.
- [x] The test should:
  - initialize a human repo with `configure_fake_human_signer()`;
  - create two commits on `agent/refactor-auth` in the agent workspace;
  - run `agd bless agent/refactor-auth --preserve` from the human checkout;
  - assert the human checkout now has three commits total: initial plus two replayed commits;
  - assert the two newest commits keep `Local Agent <agent@agd.invalid>` as author;
  - assert the two newest commits have `Human Developer <human@example.test>` as committer;
  - assert both replayed commit bodies include `AGD-Adoption: preserve`;
  - assert both replayed raw commits include a `gpgsig` header.
- [x] Run `just dev cargo test bless_preserve_replays_agent_commits_with_human_signatures`.
- [x] Expected result: fail because `--preserve` is not accepted yet.

## Task 2: CLI and Dispatch

- [x] Modify `src/cli.rs`:
  - change `Bless { branch: String }` to `Bless { branch: String, #[arg(long)] preserve: bool }`.
- [x] Modify `src/main.rs`:
  - import/use `adoption::AdoptionMode`;
  - route `--preserve` to `adoption::bless(..., AdoptionMode::Preserve)`;
  - route the default case to `AdoptionMode::Squash`.
- [x] Keep `--json` behavior unchanged; `bless` remains human output only.

## Task 3: Preserve Implementation

- [x] Modify `src/adoption.rs`:
  - introduce `pub enum AdoptionMode { Squash, Preserve }`;
  - add `pub fn bless(paths, project, branch, mode)`;
  - keep `bless_squash` as a private helper called by the shared entrypoint;
  - add a shared preparation helper that resolves the default workspace, checks clean human/agent state, acquires the lock, creates the safety ref, fetches the branch, and computes base/tip;
  - implement preserve replay:
    - enumerate commits with `git rev-list --reverse <default-target>..<fetched-ref>`;
    - fail with `selected branch has no commits to preserve` if the list is empty;
    - for each commit, run `git cherry-pick --no-commit <commit>`;
    - run `git commit -S -C <commit> --trailer AGD-Project=<id> --trailer AGD-Workspace=<id> --trailer AGD-Agent-Branch=<branch> --trailer AGD-Agent-Base=<base> --trailer AGD-Agent-Tip=<tip> --trailer AGD-Agent-Commit=<commit> --trailer AGD-Adoption=preserve`;
    - print `Preserved <branch>`.
- [x] Preserve conflict handling can remain Git's current stopped cherry-pick state for this slice; full `bless --continue/--abort` remains a later PRD item.

## Task 4: Verification

- [x] Rerun `just dev cargo test bless_preserve_replays_agent_commits_with_human_signatures`.
- [x] Run `just fmt`.
- [x] Run `just dev cargo fmt --all -- --check`.
- [x] Run `just test`.
- [x] Run `just lint`.
- [x] Run `just dev-shell 'bash scripts/manual-proof.sh'`.

## Self-Review

Spec coverage: This slice covers PRD preserve adoption mode and acceptance criterion 32. It does not implement signed merge, conflict continue/abort, patch hashes, or configurable signoff.

Placeholder scan: No TBD/TODO placeholders remain.

Type consistency: The plan uses `AdoptionMode::Squash` and `AdoptionMode::Preserve` consistently across CLI dispatch and implementation.
