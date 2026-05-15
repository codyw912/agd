# Next Phase

AGD `v0.1.0` proved the core Git authority model: agents can work in managed checkouts, commit with a local agent identity, and hand work back through explicit human adoption.

The next phase is workflow discovery. The project should learn from real usage before adding more command surface or simplifying architecture around assumptions that may not hold.

## Goals

- Dogfood AGD on real work in this repository.
- Try AGD in at least one other local repository.
- Capture repeated friction before turning it into roadmap work.
- Identify which workflows are central, which are edge cases, and which existing behavior can be removed or simplified later.
- Preserve the human approval boundary as the product's core invariant.

## Non-Goals

- Do not add new workflow flags just because they are easy to implement.
- Do not start broad architecture refactors before workflow patterns are clearer.
- Do not turn AGD into an agent harness.
- Do not expand into sandboxing, credential brokering, remote orchestration, or transcript capture in this phase.

## Discovery Workflows

Use AGD across these workflow shapes:

- Short task branch: one agent branch, one adoption PR.
- Long-running session: multiple agent branches merged into `agent/main`, then adopted.
- Human interruption: human checkout changes while agent work is in progress.
- Recovery: interrupted bless or PR operation, then continue or abort.
- Cleanup: branch/workspace cleanup after merge.
- Multi-workspace: two managed workspaces for parallel or isolated agent work.
- Stricter repo policy: explicit `--branch` and `--target` requirements.

## Observation Rule

Record observations before implementing fixes. An observation becomes roadmap work when at least one of these is true:

- It repeats across multiple sessions.
- It blocks a real workflow.
- It creates unsafe or confusing human approval behavior.
- It requires a brittle workaround.
- It exposes behavior we probably should remove instead of polish.

## Decision Cadence

Review `docs/workflow-discovery.md` after several real sessions. Convert observations into one of:

- **Workflow UX:** command naming, defaults, output, docs, or recovery guidance.
- **Safety:** authority separation, signing, push denial, protected branches, provenance, or verification.
- **Architecture:** simplification that follows stable workflow concepts.
- **Deferred:** valid idea, but not needed for the next usable release.
- **Rejected:** no longer fits the product thesis.

## Parked Architecture Questions

These are known cleanup candidates, but they should wait until workflow discovery gives better evidence:

- `src/main.rs` owns too much command orchestration.
- Workspace selection is duplicated across command modules.
- `src/git.rs` is a thin subprocess wrapper; higher-level Git intent may deserve deeper helpers.
- `tests/p0a.rs` is comprehensive but too large to navigate comfortably.

## Exit Criteria

This phase is ready to move into simplification when:

- We have several recorded workflow observations.
- The core happy path has been used outside this repository.
- The repeated pain points are clearer than the hypothetical ones.
- We can name the next simplification target in terms of workflow evidence.
