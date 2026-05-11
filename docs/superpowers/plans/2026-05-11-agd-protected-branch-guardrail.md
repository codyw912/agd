# AGD Protected Branch Guardrail Plan

## Goal

Prevent routine agent commits directly on protected branch names inside managed AGD workspaces.

## Scope

- Install a workspace `pre-commit` hook during `agd init`, workspace creation, and guardrail repair.
- Block commits on `main`, `master`, `trunk`, `develop`, `stable`, `production`, `prod`, `release/*`, `stable/*`, `production/*`, and `prod/*`.
- Keep agent feature branches such as `agent/refactor-auth` working normally.
- Extend `agd doctor` to report the protected-branch hook state.

## Steps

- [x] Add failing integration coverage for blocking commits on `main`.
- [x] Implement the workspace `pre-commit` hook in `guardrails`.
- [x] Add doctor detection for missing/stale protected-branch hook.
- [x] Run focused and full verification.
