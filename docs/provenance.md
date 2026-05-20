# Provenance

AGD provenance links human-owned adoption history back to the agent work that
produced it. Today, AGD writes local provenance records and still keeps commit
trailers for verification compatibility. The likely future direction is local
provenance by default, with committed provenance as an explicit project choice.

## Current Provenance

When AGD adopts a branch, it records AGD trailers on the human adoption commit
or commits. The trailers include:

- `AGD-Project`
- `AGD-Workspace`
- `AGD-Agent-Branch`
- `AGD-Agent-Base`
- `AGD-Agent-Tip`
- `AGD-Adoption`
- `AGD-Patch-SHA256`

Preserve adoption also records `AGD-Agent-Commit` on each replayed commit.

AGD also writes local records under `.git/agd/provenance/`:

- `records/<human-commit>.json`
- `by-agent/<agent-tip>.json`

These files are local to the developer checkout and are not committed to the
project repository.

The trailers make the current `agd verify` command possible:

```bash
agd verify <commit>
```

Verification recomputes the adopted patch and compares it to the recorded patch
hash. This proves the human adoption commit matches the agent patch AGD recorded
at adoption time.

## Why This Is Not The End State

Committed provenance is visible in remote history. That is useful for debugging
AGD itself and for AGD-native projects, but it is not always a good default for
normal project work.

AGD's product direction is developer-local:

- remote branches should look like ordinary developer branches
- remote commits should usually look human-authored
- reviewers should not need to understand AGD metadata to review a PR
- AGD should still support local verification and recovery

Those goals point toward local provenance by default.

## Local Provenance Direction

The local provenance store records adoption facts outside normal remote commit
messages. It includes:

- project id
- workspace id
- agent branch
- agent base and tip
- adoption mode
- human adoption branch
- human adoption commit or commit range
- patch hash
- adoption timestamp
- verification status

With that shape, AGD can keep verification and recovery data locally while the
remote project receives normal human-owned Git history.

See [Local Provenance Design](local-provenance-design.md) for the proposed
storage path, record schema, and migration plan.

## Committed Provenance As An Option

Some projects may want provenance committed to history. Examples:

- AGD-native repositories
- audit-heavy projects that require visible local tool metadata
- debugging AGD adoption behavior
- sharing provenance between machines

That should be an explicit mode, not a requirement for ordinary projects.

## Current Guidance

For now, expect AGD adoption commits and generated PR bodies to include
provenance details. Treat that as alpha behavior that supports verification
while the local provenance model is still being designed.

If remote history shape matters, choose adoption units carefully and review the
generated commits before pushing or merging them.
