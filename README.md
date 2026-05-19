# AGD

AGD is a local Git delegation tool for coding-agent workflows.

It gives coding agents their own Git checkout, identity, branch space, and commit authority so they can commit unattended without using your personal signing key. You review the resulting branch later and adopt it explicitly with a human-signed commit, signed merge, or pull request.

AGD is not an agent harness. Agents keep using normal Git commands inside a normal Git repository.

## Status

AGD is alpha software under active development. The core workflow is usable, but command behavior, metadata formats, and safety checks may still change as the project matures.

## Installation

AGD currently ships as a Rust CLI. Install it with Cargo:

```bash
cargo install --git https://github.com/codyw912/agd.git --locked
```

To install from a specific release tag:

```bash
cargo install --git https://github.com/codyw912/agd.git --tag v0.1.0 --locked
```

For local development from a checkout:

```bash
cargo install --path . --locked
```

AGD expects Git to be available. `agd pr` can use the GitHub CLI (`gh`) or GitLab CLI (`glab`) to open pull requests or merge requests; without those tools, AGD prints the pushed branch and a compare URL so you can finish in the browser.

## Core Workflow

Initialize AGD from your human checkout:

```bash
agd init
```

Enter the managed agent workspace:

```bash
agd shell
```

For scripts or manual directory changes, `agd path` prints the workspace path:

```bash
cd "$(agd path)"
```

Run your agent or work normally from the managed workspace:

```bash
git switch -c agent/refactor-auth
git commit -m "Extract auth retry helper"
```

For longer unattended sessions, use `agent/main` as an agent-local integration branch and merge task branches into it as work stabilizes:

```bash
git switch -c agent/main main
git switch -c agent/refactor-auth
git commit -m "Extract auth retry helper"
git switch agent/main
git merge --no-ff agent/refactor-auth
```

See [Long-Running Autonomous Sessions](docs/autonomous-sessions.md) for the
current dogfood workflow and its known review-size tradeoffs.

Review from the human checkout:

```bash
agd branches
agd log agent/refactor-auth
agd diff agent/refactor-auth
agd files agent/refactor-auth
```

See [Local Review](docs/local-review.md) for the current review workflow and
optional lazygit usage.

For additional managed workspaces, pass `--workspace <id>` from the human checkout,
or run path, status, review, sync, handoff, shell, and cleanup commands from
inside that agent workspace to target it by default.

After the human target branch moves, sync the workspace and rebase the current
agent branch from inside the agent workspace:

```bash
agd sync --rebase
```

Adopt the branch:

```bash
agd bless agent/refactor-auth
```

By default, `bless` creates a human-owned adoption branch from the configured target branch. For example, `agent/refactor-auth` adopts onto `refactor-auth`, leaving `main` ready for a normal protected-branch PR. The integration branch `agent/main` adopts onto `adopt/main` by default so it does not collide with protected `main`. Use `--workspace <id>` to adopt from a named workspace, `--branch <name>` for project branch naming policies, `--target <branch>` for a non-default base branch, or `--direct` when you explicitly want to adopt onto the current human branch.

See [Adoption Modes](docs/adoption-modes.md) for the current squash,
preserve, and merge tradeoffs.

See [Provenance](docs/provenance.md) for the current AGD trailer behavior and
the local-provenance direction.

To adopt and open the protected-branch PR in one step:

```bash
agd pr --bless agent/refactor-auth
```

Use `--branch <name>` and `--target <branch>` with `--bless` when the adoption branch name or base branch needs to follow project policy:

```bash
agd pr --bless --branch cody/refactor-auth --target release agent/refactor-auth
```

If signing or conflict resolution interrupts a blessed PR operation, resolve the issue and continue it:

```bash
agd pr --continue
```

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
agd status [--workspace <id>]
agd identity
agd doctor [--repair]
agd sync [--workspace <id>] [--rebase [branch]]
agd handoff [--workspace <id>] [--include-untracked <path>...]
agd branches [--workspace <id>]
agd log [--workspace <id>] [branch]
agd diff [--workspace <id>] [branch]
agd files [--workspace <id>] [branch]
agd bless [--workspace <id>] [--preserve | --merge] [--target <branch>] [--branch <name>] <branch>
agd bless --direct [--workspace <id>] [--preserve | --merge] <branch>
agd bless --continue
agd bless --abort
agd pr [--workspace <id>] [branch]
agd pr --bless [--workspace <id>] [--target <branch>] [--branch <name>] [branch]
agd pr --continue
agd verify <commit>
agd shell [--workspace <id>]
agd discard [--force] <branch>
agd reset-workspace [--force]
agd workspace list
agd workspace create <name>
```

Most commands also support `--json` for machine-readable output.

## Safety Model

AGD separates authority, not execution.

Agent work happens in a separate Git clone with a local agent identity and no access to your human signing key. The human approval boundary is explicit: review the branch, then bless it onto a human adoption branch, preserve it, merge it, or explicitly adopt it directly. AGD also blocks default pushes from the agent workspace and treats protected branch names such as `main`, `master`, `trunk`, `develop`, `release/*`, `stable/*`, `production/*`, and `prod/*` as non-agent branches.

Current alpha adoption commits include AGD provenance trailers for verification.
See [Provenance](docs/provenance.md) for current behavior and the local
provenance direction.

AGD does not sandbox filesystems, networks, credentials, or processes. Use separate sandboxing if you need those controls.

## Known Limitations

- AGD is alpha software. Metadata formats, command defaults, and recovery behavior may change before a stable release.
- AGD does not provide process, filesystem, network, or credential sandboxing.
- Binary release artifacts are not published yet; install with Cargo for now.
- `agd pr` needs `gh` or `glab` for automatic pull request creation. Without either tool, AGD falls back to manual next steps.
- Git LFS and submodules are detected by `agd doctor`, but AGD does not install or manage those tools for the workspace.
- AGD is designed around Git branches and pull requests. Hosts or repositories with unusual branch policies may need explicit `--branch` and `--target` options.

## Development

This repo uses `just` as the shared command entry point:

```bash
just fmt
just fmt-check
just install-local
just test
just lint
just package-check
```

For ad hoc Rust commands inside the project dev environment:

```bash
just dev cargo check
```

For shell syntax such as pipes or redirects:

```bash
just dev-shell 'cargo test -- --nocapture'
```
