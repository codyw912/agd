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
