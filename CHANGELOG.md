# Changelog

All notable changes to AGD will be documented in this file.

## Unreleased

No changes yet.

## 0.1.0 - 2026-05-15

Initial alpha release.

### Added

- Managed agent workspaces with local agent Git identity, unsigned commits, push denial, and protected-branch commit guardrails.
- Human approval workflows through `agd bless`, `agd pr --bless`, direct adoption, preserved commits, and signed merge commits.
- Review commands for agent branches: `branches`, `log`, `diff`, and `files`.
- Workspace synchronization and handoff commands for keeping agent work aligned with the human checkout.
- Multi-workspace support with workspace creation, listing, explicit `--workspace` selection, and current-workspace defaults.
- Doctor checks and repair support for project metadata, workspace markers, guardrails, operation locks, Git LFS warnings, and submodule warnings.
- Recovery commands for interrupted bless and PR operations.
- JSON output for automation-oriented commands.
- Release-readiness documentation, CI, install instructions, and known limitations.

### Known Limitations

- AGD is alpha software; command defaults and metadata formats may still change.
- AGD separates Git authority but does not sandbox execution.
- Binary release artifacts are not published yet; install with Cargo.
- Automatic PR creation depends on `gh` or `glab`; otherwise AGD prints manual next steps.
