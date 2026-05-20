# Local Provenance Design

AGD currently stores adoption provenance in commit trailers. That makes
verification possible, but it also puts AGD-specific metadata into remote
history. Local provenance moves the default record into developer-local AGD
state while keeping committed provenance available as an explicit option later.

## Goals

- Verify human adoption commits without requiring AGD trailers in remote commit
  messages.
- Preserve the link from agent work to human-owned history.
- Support squash, preserve, and merge adoption modes.
- Keep records local to the developer by default.
- Leave room for optional committed or exported provenance later.

## Non-Goals

- Do not remove current commit trailers in the first implementation slice.
- Do not introduce project-level AGD requirements.
- Do not design transcript capture or agent process auditing.
- Do not solve arbitrary range carving in the provenance store itself.

## Storage

Store local provenance under the human checkout Git metadata:

```text
.git/agd/provenance/
  records/
    <human-commit>.json
  by-agent/
    <agent-tip>.json
```

Use `.git/agd/` because it is already local to the checkout, is not committed,
and is covered by existing AGD recovery state patterns.

`records/<human-commit>.json` is the primary lookup for `agd verify <commit>`.
`by-agent/<agent-tip>.json` is an optional index for future review and recovery
commands.

## Record Shape

```json
{
  "version": 1,
  "project_id": "project_...",
  "workspace_id": "default",
  "agent_branch": "agent/refactor-auth",
  "agent_base": "<sha>",
  "agent_tip": "<sha>",
  "agent_commit": "<sha or null>",
  "adoption": "squash | preserve | merge",
  "human_branch": "refactor-auth",
  "human_commit": "<sha>",
  "patch_sha256": "<sha256>",
  "created_at": "2026-05-19T12:00:00Z"
}
```

For squash and merge adoption, `agent_commit` is `null` and `patch_sha256`
describes the patch from `agent_base` to `agent_tip`.

For preserve adoption, AGD writes one record per replayed human commit.
`agent_commit` is the source agent commit, and `patch_sha256` describes that
single adopted patch.

## Write Semantics

Write provenance records only after the human adoption commit exists.

For each record:

1. Compute or reuse the patch hash.
2. Resolve the human adoption commit.
3. Write `records/<human-commit>.json` with an atomic temp-file rename.
4. Update the `by-agent/<agent-tip>.json` index after the primary record write.

If local provenance writing fails after the adoption commit succeeds, AGD should
fail the command with recovery guidance rather than silently losing local
verification data.

## Verify Semantics

`agd verify <commit>` should eventually use this order:

1. Local provenance record for `<commit>`.
2. AGD commit trailers.
3. Missing metadata result.

During migration, trailers remain authoritative when both exist and disagree.
Once local provenance is stable, AGD can decide whether local records should
become authoritative by default.

## Recovery

Interrupted adoption currently uses `.git/agd/bless.json` and `.git/agd/pr.json`.
Local provenance should not replace those operation-state files.

Instead:

- operation state tracks an in-progress mutation
- provenance records track completed adoption commits

For squash continuation, write local provenance after `agd bless --continue`
creates the adoption commit. For PR continuation, reuse the existing completed
adoption record while retrying remote publication.

## Migration Plan

1. Write local records while keeping existing trailers unchanged. Implemented:
   AGD writes records for squash, preserve, merge, and continued squash
   adoption.
2. Teach `agd verify` to read local records as a fallback. Implemented:
   trailer metadata remains authoritative during migration, and local records
   verify commits that do not carry AGD trailers.
3. Add diagnostics in `agd doctor` for malformed local provenance.
4. Add an explicit committed-provenance mode if needed.
5. Consider making local-only provenance the default for adoption commits.

The first two migration steps are now implemented. Doctor diagnostics should
stay in a separate reviewable slice.

## Open Questions

- Should local records include the adoption command arguments?
- Should records include the remote PR URL after `agd pr --bless` succeeds?
- Should `by-agent` index by agent tip only, or by branch plus tip?
- Should local provenance be copied when a human branch is renamed?
- How should AGD garbage-collect records for deleted local branches?
