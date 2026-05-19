# Workflow Discovery

Use this log to capture AGD workflow observations before turning them into features or refactors.

## Template

```md
## Observation: <short title>

Date:
Repo/context:
Workflow:
Expected:
Actual friction:
Workaround:
Severity: low | medium | high | blocking
Repeated: yes | no
Candidate response:
Status: observed | accepted | deferred | rejected
```

## Observations

## Observation: Managed workspace pull can race human checkout sync

Date: 2026-05-15
Repo/context: AGD dogfooding in this repository after merged PRs.
Workflow: After a PR is merged, switch both the human checkout and managed workspace to `main`, then pull both checkouts.
Expected: Both checkouts fast-forward to the same merged `main` in one sync pass.
Actual friction: The managed workspace's `origin` points at the human checkout, so if the managed workspace pulls before the human checkout has fetched and fast-forwarded from GitHub, it reports `Already up to date` while still being behind the real remote. A second managed-workspace pull is needed after the human checkout updates.
Workaround: Pull the human checkout first, then pull the managed workspace. If pulling in parallel, rerun the managed workspace pull after the human checkout fast-forwards.
Severity: low
Repeated: yes
Candidate response: Document the sequencing in dogfooding notes, then watch whether this remains a manual annoyance. If it does, consider a dedicated `agd sync`/post-merge flow that updates the human checkout first and then refreshes managed workspaces in order.
Status: observed

## Observation: PR push can fail on SSH signing but recover cleanly

Date: 2026-05-15
Repo/context: AGD dogfooding in this repository while opening a blessed PR.
Workflow: Run `agd pr --bless --target main --branch <human-branch> <agent-branch>` from the human checkout.
Expected: AGD adopts the agent branch, pushes the human-owned branch, and opens a pull request.
Actual friction: The adoption succeeded, but the push step failed when the SSH/1Password agent could not sign for GitHub. The command printed a large Git/backtrace error. Running `agd pr --continue` afterward recovered the operation and opened the pull request.
Workaround: Retry with `agd pr --continue` once the SSH agent is available, or temporarily use a push URL/credential path that does not depend on the failing SSH signer.
Severity: medium
Repeated: yes
Candidate response: Keep dogfooding the recovery flow. If this keeps recurring, improve the push-failure UX by suppressing noisy backtraces, making the `agd pr --continue` next step more prominent, and documenting credential/signing recovery paths.
Status: observed

## Observation: Entering the agent workspace is not ergonomic

Date: 2026-05-17
Repo/context: Preparing to dogfood AGD on `~/dev/cs-demo-lab`.
Workflow: Initialize AGD, then enter the managed workspace to start a long-running autonomous agent session.
Expected: The transition from human checkout to managed agent workspace should be low-friction and memorable.
Actual friction: The documented flow uses `cd "$(agd path)"`, which is correct but awkward enough to interrupt setup. It also leaves open whether users should think in shell snippets, an AGD subcommand, or a mode switch.
Workaround: Use `cd "$(agd path)"`, shell aliases, or `agd shell` when an interactive shell is enough.
Severity: medium
Repeated: no
Candidate response: Dogfood both `cd "$(agd path)"` and `agd shell` before deciding. Possible directions include making `agd shell` the recommended workflow, adding a clearer command for printing/exporting workspace entry instructions, or changing `agd init` output to make the next step more actionable. Avoid making `init` itself change the caller's shell directory because child processes cannot reliably move the parent shell.
Status: observed

## Observation: Long-running squash adoption is too coarse

Date: 2026-05-19
Repo/context: Dogfooding AGD on `~/dev/cs-demo-lab` with an implementation agent using task branches merged into `agent/main`.
Workflow: Bless `agent/main`, then merge the resulting human adoption branch into `main`.
Expected: The human-owned result should preserve a normal, reviewable developer history shape.
Actual friction: Default squash adoption can turn a long autonomous session into one massive human commit. One dogfood adoption was over 42k LOC. This is tolerable in a private repo but not a good default for normal developer work or remote review.
Workaround: Use smaller task branches as adoption units, or consider `--merge`/`--preserve` where the current branch shape supports them.
Severity: high
Repeated: no
Candidate response: Revisit the default adoption model. The likely direction is that blessed history should replay and human-sign individual commits by default, with explicit flags for squash or preserving agent-authored commits. Also explore checkpointing or branch/range carving so long sessions become reviewable PR-sized units.
Status: observed

## Observation: Remote history should not require AGD provenance

Date: 2026-05-19
Repo/context: Reviewing the first large cs-demo-lab adoption and thinking about AGD as a developer-local tool.
Workflow: Adopt agent work into human-owned history for normal remote review.
Expected: Remote branches and pull requests should look like ordinary human developer work.
Actual friction: AGD provenance trailers in commit messages make remote history visibly AGD-specific. They help P0 verification, but they may confuse reviewers and make AGD feel like a project convention rather than a local developer tool.
Workaround: Accept trailers for now, or squash/rewrite manually outside AGD if the remote history must be clean.
Severity: medium
Repeated: no
Candidate response: Explore local provenance storage by default, with optional committed provenance for AGD-native projects. AGD should still be able to verify local adoptions without requiring every remote commit message to carry AGD metadata.
Status: observed

## Observation: Workspace discoverability may be improved with an in-repo symlink

Date: 2026-05-19
Repo/context: Preparing and using AGD in `~/dev/cs-demo-lab`.
Workflow: Move between the human checkout and managed agent workspace during day-to-day work.
Expected: The managed workspace should be easy to find from the project without making the project itself AGD-native.
Actual friction: Workspaces under `~/.agd/workspaces/...` preserve separation but are hard to discover and awkward to enter. Fully moving workspaces inside the repo may create nested Git and tooling problems.
Workaround: Use `agd path`, `agd shell`, or shell aliases.
Severity: medium
Repeated: yes
Candidate response: Explore creating an ignored in-repo symlink such as `.agd/default` pointing to the managed workspace. This may preserve the current external storage model while making `cd .agd/default` possible. Evaluate editor indexing, cleanup, repair, and cross-platform behavior before implementing.
Status: observed
