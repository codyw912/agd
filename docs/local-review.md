# Local Review

AGD's review workflow should help a developer decide what is ready to become
ordinary human-owned project history. The remote project should not need to know
that AGD was involved.

Use this workflow before blessing a branch, especially during a long-running
autonomous session.

## Inspect Agent Branches

From the human checkout:

```bash
agd branches
```

Use the branch list to separate three cases:

- ready task branches that can become normal pull requests
- active task branches that need more agent work
- integration branches such as `agent/main` that may be too large to review as
  one unit

## Review One Branch

Start with the branch summary:

```bash
agd log agent/refactor-auth
agd files agent/refactor-auth
```

Then inspect the patch:

```bash
agd diff agent/refactor-auth
```

A branch is a good adoption candidate when it has a clear purpose, a reviewable
diff, and a commit shape you would accept from a human developer.

See [Adoption Modes](adoption-modes.md) before choosing whether to squash,
preserve, or merge the branch.

## Use Lazygit Optionally

AGD does not require lazygit, but it can make local review easier because the
managed workspace is still a normal Git repository.

Enter the workspace:

```bash
agd shell
```

Then run lazygit:

```bash
lazygit
```

Useful lazygit checks:

- compare `agent/*` branches against `main`
- inspect commits on a task branch before blessing it
- review files changed by `agent/main`
- decide whether a task branch should be adopted separately before it is merged
  into a larger integration branch

Avoid pushing from the managed workspace. AGD configures the workspace to deny
default pushes; use `agd pr --bless` from the human checkout when work is ready
for remote review.

## Choose The Adoption Unit

Prefer the smallest branch that still represents complete work:

```bash
agd pr --bless agent/refactor-auth
```

Use `agent/main` only when the integrated session is genuinely the review unit:

```bash
agd pr --bless agent/main
```

If `agent/main` is too large, review its merged task branches and adopt those
individually when possible. AGD does not yet carve arbitrary commit ranges into
human adoption branches, so branch discipline is the current way to keep review
size under control.

## Current Limitations

Local review is still an active design track:

- AGD has review commands, but no dedicated local review UI
- lazygit integration is optional and informal
- long-running sessions can still accumulate more work than one pull request
  should contain
- future AGD workflows may help carve branch ranges or checkpoints into
  separate human-owned branches
