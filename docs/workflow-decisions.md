# Workflow Decisions

This document converts workflow discovery observations into current product
direction. It is not a permanent roadmap. Update it as dogfooding changes what
AGD needs to become.

## Accepted Direction

### AGD Is Developer-Local

AGD should behave like a developer tool, not a project convention. A developer
should be able to use AGD locally and still produce ordinary remote history:
human-owned commits, normal branch names, and reviewable pull requests.

Project-level AGD adoption can exist later, but it should not be required for
normal use.

### Agent History Is Working History

Agent commits are useful while work is in progress. The adoption boundary is
where AGD should turn that working history into human-owned project history.

This makes the history-shaping problem central:

- what branch or range is ready to adopt?
- should commits be squashed, replayed, or merged?
- should `agent/main` be adopted directly or treated as an integration branch?
- how can long sessions become normal PR-sized work?

### `agent/main` Is Integration, Not Always Review

`agent/main` is useful for long-running autonomous sessions because the agent
can merge stable task branches into a coherent local base. It should not imply
that the entire session is the right remote review unit.

Current guidance is to prefer task branches as adoption units and use
`agent/main` adoption only when the integrated session is genuinely reviewable
as one PR.

### Local Provenance Is The Likely Default

AGD needs provenance for verification and recovery, but remote history should
not require AGD-specific trailers by default. The likely end state is local
provenance by default, with committed provenance available for AGD-native
projects or explicit audit workflows.

### `agd shell` Is The Primary Entry Point

`agd shell` is the recommended interactive path into the managed workspace.
`agd path` remains the scriptable path lookup.

## Deferred Direction

### Project-Root Workspace Symlink

An ignored `.agd/default` symlink may improve discoverability, but it needs more
evidence before implementation. The unresolved risks are editor indexing,
nested project discovery, repair behavior, named workspace layout, and
cross-platform symlink support.

### Dedicated Local Review UI

Lazygit is a useful optional companion today. A first-class AGD review UI should
wait until the branch/range adoption model is clearer.

### Broad Architecture Simplification

Several modules are ready for cleanup, especially command orchestration and test
organization. Those refactors should follow stable workflow concepts rather
than anticipate them.

## Next Implementation Candidates

### 1. Preserve-Style Adoption As The Default Direction

Current squash adoption is simple but too coarse for long-running sessions.
Explore making replay-and-sign the default adoption behavior while keeping
explicit squash and merge modes.

Open design points:

- how to handle merge commits in `agent/main`
- how to preserve useful commit messages without preserving agent identity
- how to verify replayed commits when provenance moves local
- how to recover from conflicts across a replay sequence

### 2. Local Provenance Store

Design a local store that records adoption facts without requiring AGD trailers
in remote commit messages.

It should support:

- mapping agent branches, commits, or ranges to human adoption commits
- patch hash verification
- `agd verify`
- interrupted operation recovery
- optional export or committed provenance later

See [Local Provenance Design](local-provenance-design.md) for the current
implementation plan.

### 3. Reviewable Session Carving

Long-running sessions need a way to turn agent work into PR-sized human
branches. AGD may need commands or documented workflows for choosing branches or
ranges from `agent/main`.

The first implementation should probably be conservative and Git-native:

- inspect branches and commits locally
- adopt complete branches first
- avoid arbitrary range rewriting until verification and provenance semantics
  are clear

### 4. Post-Merge Sync Ergonomics

The managed workspace currently depends on the human checkout as its origin, so
post-merge sync order matters. A dedicated sync-after-merge flow may reduce the
manual sequence after repeated PR merges.

This is useful but lower risk than adoption history and provenance.

## Current Process

For the next few PRs, prefer one of these shapes:

- implement a narrow piece of the accepted direction
- write a focused design for an implementation candidate
- simplify code only when the workflow concept behind it is already stable

Avoid adding flags or workflow surface that does not directly support
developer-shaped history, local provenance, or reviewable session carving.
