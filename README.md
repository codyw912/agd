# AGD

AGD is a local Git delegation tool for coding-agent workflows.

It gives coding agents their own Git checkout, identity, branch space, and commit authority so they can commit unattended without using your personal signing key. You review the resulting branch later and adopt it explicitly with a human-signed commit, signed merge, or pull request.

AGD is not an agent harness. Agents keep using normal Git commands inside a normal Git repository.

## Status

AGD is alpha software under active development. The core workflow is usable, but command behavior, metadata formats, and safety checks may still change as the project matures.

## Core Workflow

Initialize AGD from your human checkout:

```bash
agd init
```

Move into the managed agent workspace:

```bash
cd "$(agd path)"
```

Run your agent or work normally:

```bash
git switch -c agent/refactor-auth
git commit -m "Extract auth retry helper"
```

Review from the human checkout:

```bash
agd branches
agd log agent/refactor-auth
agd diff agent/refactor-auth
agd files agent/refactor-auth
```

Adopt the branch:

```bash
agd bless agent/refactor-auth
```

By default, `bless` creates a human-owned adoption branch from the configured target branch. For example, `agent/refactor-auth` adopts onto `refactor-auth`, leaving `main` ready for a normal protected-branch PR. Use `--branch <name>` for project branch naming policies, `--target <branch>` for a non-default base branch, or `--direct` when you explicitly want to adopt onto the current human branch.

## What AGD Sets Up

`agd init` creates a managed clone under your AGD home, records project metadata, and configures the agent workspace with:

- local agent identity: `Local Agent <agent@agd.invalid>`
- commit signing disabled for normal commits
- explicit signing attempts denied and logged
- push disabled by default
- protected branch commit guardrails
- workspace metadata for discovery and repair

Your human checkout keeps its existing Git identity, signing setup, remotes, and hooks.

## Common Commands

```bash
agd init
agd path [--workspace <id>]
agd status
agd identity
agd doctor [--repair]
agd sync [--rebase <branch>]
agd handoff [--include-untracked <path>...]
agd branches
agd log [branch]
agd diff [branch]
agd files [branch]
agd bless [--preserve | --merge] [--target <branch>] [--branch <name>] <branch>
agd bless --direct [--preserve | --merge] <branch>
agd bless --continue
agd bless --abort
agd pr [branch]
agd verify <commit>
agd shell
agd discard [--force] <branch>
agd reset-workspace [--force]
agd workspace list
agd workspace create <name>
```

Most commands also support `--json` for machine-readable output.

## Safety Model

AGD separates authority, not execution.

Agent work happens in a separate Git clone with a local agent identity and no access to your human signing key. The human approval boundary is explicit: review the branch, then bless it onto a human adoption branch, preserve it, merge it, or explicitly adopt it directly. AGD also blocks default pushes from the agent workspace and treats protected branch names such as `main`, `master`, `trunk`, `develop`, `release/*`, `stable/*`, `production/*`, and `prod/*` as non-agent branches.

AGD does not sandbox filesystems, networks, credentials, or processes. Use separate sandboxing if you need those controls.

## Development

This repo uses `just` as the shared command entry point:

```bash
just fmt
just test
just lint
```

For ad hoc Rust commands inside the project dev environment:

```bash
just dev cargo check
```

For shell syntax such as pipes or redirects:

```bash
just dev-shell 'cargo test -- --nocapture'
```
