# Long-Running Autonomous Sessions

This guide describes the current dogfood workflow for letting an agent work for
an extended period while keeping human-owned upstream history reviewable.

AGD does not run or supervise the agent process. Start the agent from the
managed workspace and let normal Git history carry the work.

## Start The Session

From the human checkout:

```bash
agd init
agd shell
```

Inside the managed workspace, create an agent-local integration branch:

```bash
git switch -c agent/main main
```

Use task branches for individual chunks of work:

```bash
git switch -c agent/refactor-auth
git commit -m "Extract auth retry helper"
```

When a task branch is stable enough for the agent to build on, merge it into
`agent/main`:

```bash
git switch agent/main
git merge --no-ff agent/refactor-auth
```

`agent/main` is a local integration branch for the autonomous session. It lets
the agent continue from a coherent combined state without requiring every task
branch to be adopted immediately.

## Sync Human Changes

When the human target branch moves, update the human checkout first:

```bash
git switch main
git pull --ff-only
```

Then sync the managed workspace. From inside the agent workspace, this also
rebases the current agent branch:

```bash
agd sync --rebase
```

If you are on `agent/main`, that rebases the integration branch. If you are on a
task branch, that rebases the task branch. To rebase a specific branch from the
human checkout, pass it explicitly:

```bash
agd sync --rebase agent/refactor-auth
```

## Review During The Session

Review from the human checkout whenever a task branch looks ready:

```bash
agd branches
agd log agent/refactor-auth
agd diff agent/refactor-auth
agd files agent/refactor-auth
```

Prefer adopting task branches while they are still reviewable:

```bash
agd pr --bless agent/refactor-auth
```

This keeps remote pull requests shaped like normal developer work. Waiting until
the end and blessing all of `agent/main` can produce a very large adoption.

## Adopt The Integration Branch

Use `agent/main` adoption when the integrated session is genuinely the right
review unit:

```bash
agd pr --bless agent/main
```

By default, `agent/main` adopts onto `adopt/main` so it does not collide with a
protected `main` branch.

For stricter branch naming policies, choose the human branch name explicitly:

```bash
agd pr --bless --branch cody/session-checkpoint agent/main
```

## Current Limitations

AGD can support long-running autonomous sessions today, but the workflow still
needs refinement:

- default squash adoption can turn a long session into one very large human commit
- preserving useful agent commit history while keeping human-owned history clean
  is still an active design track
- `agent/main` is useful for agent-side integration, but it is not always the
  right remote review unit
- AGD does not yet help carve ranges from a long session into multiple human
  adoption branches

Until those pieces mature, treat task branches as the preferred adoption unit
and `agent/main` as the agent's local integration branch.
