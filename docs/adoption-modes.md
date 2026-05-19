# Adoption Modes

AGD adoption is the step where agent-local Git history becomes human-owned Git
history. The human checkout keeps its normal identity, signing configuration,
branch protections, and remote workflow.

The current alpha supports three adoption modes. They are useful in different
review situations, and the defaults may change as AGD learns more from
dogfooding.

## Squash Adoption

Squash adoption is the current default:

```bash
agd pr --bless agent/refactor-auth
```

It creates one human-owned adoption commit on the human adoption branch. This is
simple and easy to verify, but it can hide useful agent commit structure.

Use squash adoption when:

- the branch is small
- the agent made noisy intermediate commits
- the final diff is more important than the intermediate history

Avoid squash adoption for large autonomous sessions. A long `agent/main` session
can become one very large human commit.

## Preserve Adoption

Preserve adoption replays the agent commits as human-owned commits:

```bash
agd pr --bless --preserve agent/refactor-auth
```

This keeps commit granularity while making the adopted commits belong to the
human checkout. It is often a better fit when the agent has already produced
logical commits.

Use preserve adoption when:

- the agent branch has reviewable commits
- each commit builds cleanly on the target branch
- preserving the branch story matters for future debugging

Preserve adoption still does not solve pull request size by itself. A huge
branch with many logical commits can still be too large to review as one PR.

## Merge Adoption

Merge adoption creates a signed human merge commit while leaving the agent
commits reachable:

```bash
agd pr --bless --merge agent/refactor-auth
```

This keeps the original agent commits intact behind a human-owned merge. It is
useful when preserving exact agent commit objects matters locally, but it makes
agent identity visible in history.

Use merge adoption when:

- preserving exact agent commits is important
- the project accepts merge commits for feature branches
- visible agent-authored commits are acceptable for the repository

Avoid merge adoption when remote history should look indistinguishable from
normal human-authored project history.

## Choosing A Mode

For normal review, prefer the smallest complete branch first. Then choose the
mode based on the branch shape:

- small or noisy branch: use the default squash mode
- logical commit series: use `--preserve`
- exact agent commit preservation: use `--merge`

For long-running sessions, the adoption unit usually matters more than the mode.
Adopting task branches separately is often better than adopting all of
`agent/main` at once.

## Current Direction

AGD is moving toward developer-shaped remote history: human-owned commits,
ordinary branch names, reviewable pull requests, and AGD metadata kept local by
default where possible.

That means future defaults may favor replaying and signing logical commits
instead of squashing. Until that design is settled, choose adoption modes
explicitly when history shape matters.
