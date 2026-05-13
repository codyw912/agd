PRD v0.3 — AGD / Agent Git Delegation

1. Summary

AGD is a local Git delegation tool for coding-agent workflows.

It gives agents their own Git checkout, identity, branch space, and commit authority, so they can commit freely without using the developer’s personal signing key. The developer later reviews and adopts the work through an explicit human approval step: squash/sign, preserve/re-sign, signed merge, or PR.

The immediate goal is to remove the bottleneck caused by 1Password SSH signing prompts during unattended agent work, while preserving a clean distinction between:

agent produced this work
human approved/adopted this work

⸻

2. Core thesis

Agents should be able to create meaningful Git history autonomously.

They should not sign commits as the human developer.

The human approval boundary should happen later:

agent works in managed checkout
agent commits as agent
agent does not use human signing key
human reviews branch
human blesses/adopts/signs/merges/opens PR

AGD is not an agent harness. Agents continue using normal Git.

⸻

3. Product scope

P0 is about

Git identity separation
commit-signing delegation
managed agent checkout
agent branch review
safe adoption into human checkout
push denied by default
recoverable bless/sync operations

P0 is not about

filesystem sandboxing
network sandboxing
credential broker
VM/container runtime
agent orchestration
prompt management
remote automation
malicious-code containment
audit transcript capture
multi-user enterprise policy

Sandboxing may become a later separate layer, but the first product is a Git authority and workflow boundary.

⸻

4. Primary user

A developer using local coding agents such as:

Claude Code
Codex
Aider
Cursor agents
custom harnesses

The user wants to:

run agents normally
let agents commit while unattended
avoid approving every commit-signing prompt
avoid giving agents the personal signing key
review and sign/adopt later

⸻

5. Core model

AGD manages one or more agent workspaces for a project.

Each agent workspace is an independent Git clone with its own refs, config, remotes, and branch namespace.

human checkout
  /Users/me/dev/project
  normal Git identity
  normal signing config
  1Password SSH signing enabled
  source of truth for adoption
agent workspace
  ~/.agd/workspaces/project-a13f92c4/default
  independent Git clone
  agent Git identity
  signing disabled by default
  push denied by default
  agent can branch/commit freely

The agent workspace is a normal Git repository. Agents and harnesses should not need to know AGD exists.

Typical usage:

cd ~/dev/project
agd init
cd "$(agd path)"
claude

# or codex, aider, custom harness, etc

Later:

cd ~/dev/project
agd branches
agd diff agent/refactor-auth
agd bless agent/refactor-auth

AGD should not require launching agents through AGD. Future convenience commands may exist, but they are not core behavior.

⸻

6. Locked implementation decisions

Language: Rust
Git backend: system git subprocesses
Workspace primitive: independent managed clone
Workspace location: common user-local directory by default
Agent identity: local-only unsigned identity by default
Agent email: <agent@agd.invalid>
Push policy: denied by default
Adoption default: squash + human signature
Preserve mode: supported
Signed merge mode: supported
PR mode: late P0 / P1
Sandboxing: deferred

⸻

7. Language and implementation stack

Core language

AGD will be implemented in Rust.

Rationale:

single native CLI binary
good subprocess orchestration
strong error handling
good data modeling
good future TUI path
good future sandbox/process-control fit

Git interaction

AGD should shell out to the installed git binary.

Do not use a Git library as the source of truth in P0.

Reason: AGD must match the user’s real Git behavior for:

commit signing
config precedence
hooks
merge behavior
conflict behavior
fetch/push behavior
Git version behavior

Potential later use of Git libraries:

fast read-only inspection
TUI branch/status rendering
diff parsing
repo metadata caching

But adoption, signing, merge, fetch, push, and conflict flows should remain system-Git-driven initially.

Initial Rust dependencies

Likely starting stack:

clap        CLI parsing
serde       data model serialization
serde_json  JSON output
toml        config files
anyhow      app-level error handling
thiserror   typed internal errors
camino      UTF-8 paths
dirs        user config/cache paths
uuid        project/workspace IDs
time        timestamps
sha2        provenance hash

Potential later:

ratatui     TUI
crossterm   terminal backend
gix         read-only Git acceleration
ignore      file/path filtering

⸻

8. Workspace location

Default layout

Use a common user-local directory:

~/.agd/
  projects/
    <project-id>/
      project.json
      locks/
      operations/
  workspaces/
    <repo-slug>-<project-short-id>/
      default/
      <other-workspace>/
  logs/
    signing-denials.jsonl
  bin/
    agd-deny-signer

Example:

~/.agd/workspaces/project-a13f92c4/default

Why not in-repo by default

keeps human repo clean
avoids accidental commits of AGD state
avoids nested repo weirdness
makes reset/discard simple
supports many projects consistently
keeps agent checkout physically separate

Local project marker

AGD may store a local, uncommitted marker inside the human repo’s Git metadata:

.git/agd/project.json

Purpose:

idempotent init
project lookup from human checkout
repair after path changes
avoid repo-tree pollution

This file is not committed.

Optional convenience symlink

Later, AGD may support:

.agent -> ~/.agd/workspaces/project-a13f92c4/default

If created, it should be added to:

.git/info/exclude

not committed .gitignore, unless the user explicitly requests repo-shared config.

⸻

9. Project and workspace metadata

AGD must support multiple workspaces per project in the data model from day one, even if P0 primarily exposes the default workspace.

{
  "project_id": "project_123",
  "name": "project",
  "human_checkout": "/Users/me/dev/project",
  "default_target": "main",
  "default_workspace": "default",
  "created_at": "2026-05-08T00:00:00Z",
  "agent_identity": {
    "name": "Local Agent",
    "email": "<agent@agd.invalid>",
    "signing": "unsigned"
  },
  "workspaces": [
    {
      "id": "default",
      "name": "default",
      "kind": "managed_clone",
      "path": "/Users/me/.agd/workspaces/project-a13f92c4/default",
      "status": "active",
      "created_at": "2026-05-08T00:00:00Z"
    }
  ]
}

This avoids a later schema migration when users want concurrent agents.

P0 may expose only:

agd path

which returns the default workspace.

P1 may add:

agd workspace list
agd workspace create <name>
agd path --workspace <name>

⸻

10. Workspace marker protocol

Each agent workspace should contain a marker:

<workspace>/.agd/workspace.json

and .agd/ should be added to that workspace’s:

.git/info/exclude

Example marker:

{
  "kind": "agd-workspace",
  "project_id": "project_123",
  "workspace_id": "default",
  "human_checkout": "/Users/me/dev/project",
  "created_at": "2026-05-08T00:00:00Z",
  "agent_identity": {
    "name": "Local Agent",
    "email": "<agent@agd.invalid>",
    "signing": "unsigned"
  }
}

Purpose:

agd status discovery
shell prompt integration
TUI/IDE extension support
future harness/plugin detection
human clarity

Future agd shell / agd run may also set:

AGD_WORKSPACE=1
AGD_PROJECT_ID=project_123
AGD_WORKSPACE_ID=default

but environment variables are convenience only. The marker file is the durable discovery mechanism.

⸻

11. Project initialization

Command:

agd init

Behavior:

1. Detect current Git repo root.
2. Read current branch/remotes.
3. Generate or recover project ID.
4. Create user-local project metadata.
5. Create local human-checkout marker if appropriate.
6. Create independent managed clone from human checkout.
7. Configure agent identity.
8. Disable personal signing in agent workspace.
9. Install signing/push guardrails.
10. Disable push by default.
11. Create workspace marker.
12. Print agent workspace path.

Default output:

Initialized AGD.
Human checkout:
  /Users/me/dev/project
Agent workspace:
  ~/.agd/workspaces/project-a13f92c4/default
Agent identity:
  Local Agent <agent@agd.invalid>
Signing:
  agent commits unsigned
  explicit signing denied
Push:
  disabled by default
Use your agent normally from:
  ~/.agd/workspaces/project-a13f92c4/default

Idempotence

agd init must be idempotent.

If already initialized:

detect existing project
validate metadata
validate workspace
run doctor checks
report drift
offer repair

It must not silently create duplicate project records for the same human checkout.

⸻

12. Managed clone behavior

The default workspace primitive is an independent clone, not a linked Git worktree.

Default clone command should use normal local clone optimization:

git clone /Users/me/dev/project ~/.agd/workspaces/project-a13f92c4/default

Do not use by default:

--no-hardlinks
--shared
long-lived --reference alternates

Rationale:

normal local clone optimization reduces object duplication
independent clone preserves separate refs/remotes/config/hooks
--shared and long-lived alternates create source-object dependency risks
--no-hardlinks increases cost unnecessarily for the default case

Future workspace backends may include:

linked worktree backend for low-disk monorepo workflows
sparse checkout backend
partial clone backend
reference + dissociate clone mode

But independent clone remains the default because it provides stronger Git-state isolation.

Agent remote

Default agent workspace remote:

[remote "origin"]
  url = /Users/me/dev/project
  pushurl = agd-deny://push-disabled

Meaning:

agent workspace can fetch from human checkout
agent workspace cannot push by default
agent does not need GitHub credentials
human checkout remains source of truth

AGD should store the original upstream remote in metadata but should not expose it as the agent’s writable remote by default.

⸻

13. Config

Default

P0 should not require repo-local config.

Default project state lives outside the repo:

~/.agd/projects/<project-id>/project.json

Optional repo config

Later:

agd init --repo-config

may create:

.agd.yaml

Possible .agd.yaml:

version: 1
workspace:
  mode: managed_clone
  root: "~/.agd/workspaces"
  default: default
agent_identity:
  name: "Local Agent"
  email: "<agent@agd.invalid>"
  signing: unsigned
branches:
  preferred_prefix: "agent/"
  protected:
    - main
    - master
    - trunk
    - develop
    - release/**
    - stable/**
    - production/**
    - prod/**
push:
  default: deny
  allow: []
adoption:
  default: squash_signed
  allow:
    - squash_signed
    - preserve_signed
    - signed_merge
    - pr
  squash_author: human
  signoff: false
  require_signed_commits_on_target: unknown
sync:
  source: human_checkout
  mirror_branches:
    - main
guardrails:
  disable_personal_signing: true
  deny_explicit_signing: true
  block_protected_pushes: true
  warn_on_protected_branch_commits: true

⸻

14. Agent identity

Default agent identity:

[user]
  name = Local Agent
  email = <agent@agd.invalid>

Default signing:

[commit]
  gpgsign = false
[tag]
  gpgSign = false

Expected behavior:

git commit -m "Refactor auth client"

inside the agent workspace:

uses Local Agent identity
does not trigger 1Password
does not use human signing key
creates an unsigned commit by default

⸻

15. Signing guardrails

P0 should make accidental personal signing fail fast.

AGD should configure the agent workspace to avoid inherited human signing behavior:

git config user.name "Local Agent"
git config user.email "<agent@agd.invalid>"
git config commit.gpgsign false
git config tag.gpgSign false
git config --unset-all user.signingkey || true
git config --unset-all gpg.format || true
git config gpg.ssh.program "$AGD_DENY_SIGNER"
git config gpg.program "$AGD_DENY_SIGNER"

agd-deny-signer should:

write denial message to stderr
exit nonzero
append redacted event to ~/.agd/logs/signing-denials.jsonl

Expected behavior:

git commit -m "work"

# succeeds, unsigned, no 1Password prompt

git commit -S -m "signed work"

# fails fast, no 1Password prompt

Example denial log:

{
  "time": "2026-05-08T12:00:00Z",
  "project_id": "project_123",
  "workspace_id": "default",
  "cwd": "/Users/me/.agd/workspaces/project-a13f92c4/default",
  "message": "explicit signing attempt denied"
}

Do not log raw environment variables or secrets.

⸻

16. Push guardrails

Push is denied by default.

Primary defense:

git remote set-url --push origin agd-deny://push-disabled

Secondary UX aid:

pre-push hook with friendly message

Default behavior:

git push origin main

# fails

git push origin agent/foo

# fails unless policy explicitly allows it

Important: hooks are convenience, not enforcement. The disabled push URL is the primary guardrail.

Longer term, AGD should own controlled PR creation:

agd pr agent/foo

so the agent itself does not need GitHub credentials.

⸻

17. Branch behavior

Agent sessions may span multiple branches.

AGD should not model:

one session = one branch

Instead:

one agent workspace may contain many agent branches
review/adoption operates on selected branch or commit range

Example branch graph:

main
agent/main
agent/refactor-auth
agent/add-tests
agent/fix-lint
scratch/experiment

Agents can use normal Git:

git switch -c agent/main main
git switch -c agent/refactor-auth
git commit -m "Extract retry helper"
git switch agent/main
git merge --no-ff agent/refactor-auth
git switch -c agent/add-tests

`agent/main` is an agent-local integration branch, not the upstream target branch.
Long-running unattended sessions can merge individual `agent/*` task branches into `agent/main`
without giving the agent upstream push authority.
When adopted, `agd bless agent/main` should default to a human adoption branch such as `adopt/main`,
not `main`, so protected-target workflows still go through explicit human review and PR merge.

AGD commands default to the current branch when run inside an agent workspace.

Preferred branch prefix:

agent/

but AGD should not require it for basic operation.

⸻

18. Protected branch guardrails

Protected-by-default branch patterns:

main
master
trunk
develop
release/**
stable/**
production/**
prod/**

Inside the agent workspace, this is a guardrail, not a hard security boundary.

P0 behavior:

warn or block commits directly on protected branch names
block configured pushes to protected branches
prefer agent/* branch creation
doctor reports unsafe branch/push config

These patterns must be configurable.

⸻

19. Sync model

The human checkout owns real remote sync.

Recommended flow:

remote GitHub/GitLab
  → human checkout via normal human-authenticated fetch/pull
  → AGD syncs agent workspace from human checkout

Command:

agd sync

Sync contract

agd sync should:

fetch from human checkout into agent workspace
only update configured mirror branches
fast-forward mirror branches only
never touch agent/*
never rebase agent branches unless explicitly requested
refuse when relevant workspace is dirty

Default mirror branches:

main, if present
configured default target branch

If a mirror branch cannot fast-forward:

refuse
explain why
suggest explicit repair/reset command

Optional explicit rebase:

agd sync --rebase agent/refactor-auth

Behavior:

fast-forward mirror branch
then rebase selected agent branch onto updated target

No silent merges.

⸻

20. Dirty state policy

Dirty human checkout

agd init, agd sync, and agd bless should warn or fail if the human checkout is dirty.

Default rule:

only committed human state syncs to the agent workspace

Do not copy uncommitted human changes by default.

Dirty agent workspace

agd bless should refuse if the selected branch has uncommitted changes.

Default rule:

branch tip must represent the state being blessed

agd sync should refuse if it would touch a dirty currently checked-out branch.

agd discard should warn if the selected branch or workspace has uncommitted changes.

Explicit handoff

Common workflow:

human sketches messy uncommitted work
agent finishes it

Support this explicitly later, not as default sync behavior.

Late P0 / P1 command:

agd handoff

or:

agd sync --include-uncommitted

Behavior:

requires clean agent workspace
exports human uncommitted diff
applies it to agent workspace
copies selected untracked files only with confirmation
records handoff metadata
does not commit automatically

⸻

21. Review commands

Commands should work from either the human checkout or agent workspace.

Required:

agd path
agd status
agd branches
agd log [branch]
agd diff [branch]
agd files [branch]
agd doctor

Example from human checkout:

Project: project
Mode: human checkout
Human checkout:
  /Users/me/dev/project
Agent workspaces:
  default  ~/.agd/workspaces/project-a13f92c4/default
Pending agent branches:
  agent/refactor-auth    7 commits    14 files changed
  agent/add-tests        3 commits     5 files changed
Default target:
  main

Example from agent workspace:

Project: project
Mode: agent workspace
Workspace: default
Identity:
  Local Agent <agent@agd.invalid>
Signing:
  disabled
Push:
  denied
Current branch:
  agent/refactor-auth
Human checkout:
  /Users/me/dev/project

⸻

22. Adoption modes

22.1 Default: squash and sign

Command:

agd bless agent/refactor-auth

Behavior:

1. Acquire project adoption lock.
2. Require clean human checkout.
3. Require clean selected agent branch/workspace.
4. Create safety ref in human checkout.
5. Fetch selected agent branch into human checkout under temporary AGD ref.
6. Apply changes onto target branch as squash.
7. Create one human-signed commit.
8. Add AGD provenance trailers.
9. Release lock.

Possible implementation:

git -C "$human" fetch "$agent_workspace" \
  refs/heads/agent/refactor-auth:refs/agd/agent/refactor-auth
git -C "$human" merge --squash refs/agd/agent/refactor-auth
git -C "$human" commit -S

Default authorship:

human is author
human is committer
human signs commit

Rationale: the squash commit represents the human-approved final patch.

Optional config:

adoption:
  squash_author: human | agent

Success condition:

agent made many unsigned commits
human signs exactly one final adopted commit

⸻

22.2 Preserve and re-sign

Command:

agd bless agent/refactor-auth --preserve

Behavior:

1. Acquire project adoption lock.
2. Fetch selected agent branch.
3. Compute commits unique to agent branch.
4. Replay/cherry-pick onto target.
5. Preserve original agent author.
6. Human becomes committer.
7. Human signs each resulting commit.
8. Add provenance where practical.

This may trigger one signing confirmation per commit. That is acceptable because it happens during review/adoption, not while the agent is working unattended.

Do not add Signed-off-by by default. That has project/legal meaning.

Optional config:

adoption:
  signoff: true

⸻

22.3 Signed merge

Command:

agd bless agent/refactor-auth --merge

Behavior:

1. Fetch selected agent branch.
2. Merge with --no-ff.
3. Human signs merge commit.
4. Agent commits remain unchanged.

If target policy requires every commit to be signed and the agent commits are unsigned:

refuse by default
suggest --preserve
allow override only with explicit --force if configured

If target policy is unknown:

warn and require confirmation

⸻

22.4 PR mode

Late P0 / P1.

Command:

agd pr agent/refactor-auth

Behavior:

AGD pushes selected branch
AGD opens PR
agent itself does not own push credentials

Minimal implementation:

use gh if available for GitHub
use glab if available for GitLab
otherwise push branch and print next-step URL/instructions

PR body should include:

summary
base commit
agent branch
commit list
changed files
adoption recommendation
test evidence if available
provenance trailers/checksums

PR mode is important, but not required for the first local workflow.

⸻

23. Adoption locking and recovery

AGD must treat adoption as a recoverable operation.

Locking

During:

bless
sync
discard
reset-workspace
repair

AGD should acquire a project/workspace lock.

Lock metadata:

{
  "operation_id": "op_123",
  "operation": "bless",
  "project_id": "project_123",
  "workspace_id": "default",
  "pid": 12345,
  "started_at": "2026-05-08T12:00:00Z"
}

Safety refs

Before mutating the human checkout during adoption, AGD should create a safety ref:

refs/agd/safety/<operation-id>

Conflict handling

If adoption conflicts:

leave clear state
print exact next steps
support continue/abort
do not silently reset user work

Commands:

agd bless --continue
agd bless --abort

agd doctor should detect incomplete adoption states.

⸻

24. Provenance

Adoption commits should include machine-readable trailers.

Example:

AGD-Project: project_123
AGD-Workspace: default
AGD-Agent-Branch: agent/refactor-auth
AGD-Agent-Base: abc123
AGD-Agent-Tip: def456
AGD-Adoption: squash
AGD-Patch-SHA256: <hash>

Patch hash should be computed from a stable AGD-defined diff serialization, for example:

base tree → agent tip tree
binary-aware
full-index
path-stable

P1 command:

agd verify <commit>

Behavior:

read AGD trailers
locate metadata/workspace if available
recompute patch hash
confirm adopted diff matches agent branch/base
report verified / missing metadata / mismatch

Trailers are not a security boundary by themselves. They become useful when paired with a signed human adoption commit and optional verification.

⸻

25. Submodules and Git LFS

AGD must detect and warn about submodules and Git LFS.

Submodules

Detect:

.gitmodules

P0 behavior:

warn if submodules exist
offer/configure submodule mode

Possible config:

submodules:
  mode: none | init | recursive

Default:

warn, do not silently recurse unless configured

Git LFS

Detect:

Git LFS filters
tracked LFS pointer files
git-lfs availability

P0 behavior:

warn if repo uses LFS and git-lfs is unavailable
do not assume agent workspace can fetch new LFS objects without credentials
prefer using already-materialized files from human checkout where possible

AGD should not silently create a workspace that looks valid but has missing LFS content.

⸻

26. Hooks and pre-commit frameworks

Client-side Git hooks are not reliably copied or preserved across clones.

Agent commits may run fewer checks than human commits.

This is acceptable for agent velocity, but AGD should document it clearly.

Intended behavior:

agent commits are draft/provisional work
human adoption occurs in human checkout
human adoption runs human checkout hooks/config

Examples:

pre-commit
Husky
lefthook
custom .git/hooks scripts

If AGD installs hooks, they are workflow UX aids, not security controls.

Push URL disabling remains the primary push guardrail.

⸻

27. Doctor and repair

Command:

agd doctor

Should check:

human checkout exists
agent workspace exists
workspace is independent clone
workspace marker exists
agent identity is configured
commit signing is disabled
deny signer is configured
push URL is disabled
pre-push hook exists if expected
human checkout signing remains unchanged
remote/source config is sane
metadata paths match reality
submodules/LFS status is known
dirty state is reported
stale Git lock files are detected
incomplete merge/cherry-pick/rebase states are detected
incomplete AGD operations are detected

Possible detected Git state:

.git/index.lock
MERGE_HEAD
CHERRY_PICK_HEAD
REBASE_HEAD
AGD operation lock
broken agent origin URL
moved human checkout
missing deny signer
missing workspace marker

Repair command:

agd doctor --repair

or later:

agd repair

Repair should handle:

human checkout path moved
agent origin points to old human checkout
deny signer path moved
hook path stale
workspace marker missing
metadata drift
stale AGD operation locks

It should not blindly delete Git lock files without confirmation.

⸻

28. Discard and reset

Discard a branch:

agd discard agent/refactor-auth

Behavior:

delete selected branch from agent workspace
warn if branch/workspace is dirty
leave human checkout untouched

Reset whole workspace:

agd reset-workspace

Behavior:

archive or remove selected agent workspace
recreate clean managed clone from human checkout
recreate workspace marker
reapply identity/signing/push guardrails

⸻

29. JSON interface

AGD should support machine-readable output from the start.

agd status --json
agd branches --json
agd path --json
agd files --json
agd doctor --json

This enables future:

TUI
IDE extension
harness plugin
Claude/Codex skills
shell prompt integration
Entire/audit integration

Plugins should not parse human-readable terminal output.

⸻

30. CLI surface

P0

agd init
agd path
agd status
agd branches
agd log [branch]
agd diff [branch]
agd files [branch]
agd sync
agd bless [branch]
agd bless [branch] --preserve
agd bless [branch] --merge
agd bless --continue
agd bless --abort
agd discard [branch]
agd reset-workspace
agd doctor
agd doctor --repair

Late P0 / P1

agd pr [branch]
agd verify <commit>
agd handoff
agd workspace list
agd workspace create <name>
agd path --workspace <name>
agd identity
agd open
agd shell
agd tui

Later convenience only

agd run -- <agent>
agd exec -- <command>

These should not define the product model.

⸻

31. Identity modes

P0: unsigned-local

Default.

agent commits are unsigned
agent does not use human key
human signs adoption commit

Identity:

Local Agent <agent@agd.invalid>

P1: session-key

Optional local provenance.

AGD creates ephemeral/session signing key
agent commits are signed by session key
AGD records public key in metadata
GitHub verification not required

P1/P2: project-agent-key

For mature projects.

stable project-level agent signing key
possibly registered with GitHub/GitLab bot identity
agent commits can be verified as agent

P2: bot/app

For serious automation.

GitHub App / bot identity
AGD controls PR creation
agent branch pushes are policy-bound

⸻

32. Guardrail model

AGD protects against:

agent accidentally using human Git identity
agent accidentally invoking personal commit signing
agent triggering repeated 1Password signing prompts
agent pushing to protected branches by default
agent branch changes mutating human checkout refs
unclear review/adoption boundary

AGD does not protect against:

malicious same-user process reading other files
agent cd'ing into the human checkout
agent bypassing Git hooks manually
network exfiltration
credential theft outside Git signing
arbitrary host mutation

Docs should state this clearly.

Future sandboxing or harness-level permissions can harden execution, but P0 is a Git authority separation tool.

⸻

33. Internal Rust module shape

Initial simple crate:

src/
  main.rs
  cli.rs
  config.rs
  paths.rs
  project.rs
  workspace.rs
  git.rs
  identity.rs
  guardrails.rs
  adoption.rs
  sync.rs
  doctor.rs
  repair.rs
  provenance.rs
  locks.rs
  output.rs
  errors.rs

Possible later split:

crates/
  agd-core/
  agd-cli/
  agd-tui/

Do not split too early.

Core types:

struct Project {
    id: ProjectId,
    name: String,
    human_path: Utf8PathBuf,
    default_target: String,
    default_workspace: WorkspaceId,
    workspaces: Vec<Workspace>,
    agent_identity: Identity,
}
struct Workspace {
    id: WorkspaceId,
    name: String,
    path: Utf8PathBuf,
    kind: WorkspaceKind,
    status: WorkspaceStatus,
}
struct Identity {
    name: String,
    email: String,
    signing: SigningMode,
}
enum SigningMode {
    Unsigned,
    SessionKey,
    ProjectAgentKey,
}
enum AdoptionMode {
    SquashSigned,
    PreserveSigned,
    SignedMerge,
}
struct Git {
    dir: Utf8PathBuf,
}

All Git commands should be run as:

git -C <dir> ...

and return structured output/errors.

⸻

34. Testing strategy

Use real temporary Git repos. Do not heavily mock Git behavior.

Test fixtures should create:

human repo
agent workspace
fake personal signer
AGD deny signer
fake local remote
branches with commits
dirty checkouts
conflicts
submodule repo
LFS-like pointer files
stale lock files

Critical tests:

1. agd init creates independent agent workspace.
2. agd init is idempotent.
3. workspace marker is created and ignored.
4. agent normal commit uses Local Agent identity.
5. agent normal commit does not invoke fake personal signer.
6. agent git commit -S invokes AGD deny signer, not personal signer.
7. deny signer writes to stderr and logs redacted event.
8. push to origin fails by default.
9. fetch/sync from human checkout works.
10. sync only fast-forwards mirror branches.
11. sync never touches agent/* by default.
12. agent branches do not mutate human checkout branches.
13. agd branches lists agent branches.
14. agd diff shows target-vs-agent changes.
15. agd bless creates one signed human commit.
16. agd bless --preserve creates signed rewritten commits.
17. agd bless --merge creates signed merge commit.
18. dirty human checkout blocks adoption/sync.
19. dirty agent workspace blocks bless.
20. conflicts are recoverable with bless --abort / --continue.
21. AGD lock prevents concurrent bless operations.
22. agd doctor detects unsafe signing config.
23. agd doctor detects unsafe push config.
24. agd doctor detects stale/incomplete Git operation state.
25. agd doctor --repair fixes moved human checkout path.
26. submodules are detected and warned.
27. LFS usage is detected and warned.

Signing tests should use fake signer binaries that write to temp files, proving which signer was invoked.

⸻

35. Manual proof script

Before full implementation, include:

scripts/manual-proof.sh

Purpose:

validate the core workflow
act as onboarding example
seed integration tests
clarify expected Git commands

Script should demonstrate:

create independent clone
configure agent identity
disable signing
deny explicit signing
disable push
create agent commits
squash/sign into human checkout

⸻

36. Naming

Current internal name:

AGD = Agent Git Delegation

Possible external names:

delegate
git-delegate
steward
ward
branchyard
agent-delegate

Current recommendation:

Use agd internally while building.
Do not over-invest in branding yet.
Consider delegate or git-delegate once the surface stabilizes.

Command examples with a future name:

delegate init
delegate path
delegate branches
delegate bless agent/refactor-auth

The name should leave room for later sandboxing/audit features without overpromising them now.

⸻

37. P0 acceptance criteria

Initialization

1. agd init works in a Git repo.
2. agd init is idempotent.
3. AGD creates user-local project metadata.
4. AGD creates independent managed clone.
5. AGD creates workspace marker.
6. agd path returns the default agent workspace path.
7. agd status identifies human vs agent workspace mode.

Identity/signing

8. Human checkout signing remains unchanged.
9. Agent workspace normal commits use Local Agent <agent@agd.invalid>.
10. Agent workspace normal commits do not invoke 1Password.
11. Explicit signed commits fail fast without invoking personal signer.
12. Denied signing attempts are logged.
13. Agent workspace does not inherit human signing key for normal commits.

Branching

14. Agent can create/switch/merge/delete branches normally.
15. Agent branch changes do not mutate human checkout refs.
16. AGD can list candidate agent branches.
17. Multiple workspaces are supported in metadata.

Push

18. git push from agent workspace fails by default.
19. protected branch push is blocked.
20. push URL guardrail is primary.
21. agd doctor reports unsafe push config.

Sync

22. agd sync updates configured mirror branches only.
23. agd sync fast-forwards only.
24. agd sync never touches agent/* by default.
25. dirty relevant workspace causes sync refusal.

Review

26. agd diff shows target-vs-agent changes.
27. agd log shows commits unique to selected agent branch.
28. agd files shows changed files.

Adoption

29. agd bless squashes selected branch into human target.
30. Squash adoption creates one human-signed commit.
31. Squash adoption includes AGD provenance trailers.
32. agd bless --preserve replays commits with human signatures.
33. agd bless --merge creates a signed merge commit.
34. Conflicts are recoverable with bless --abort / --continue.
35. Adoption never requires the agent to access the human signing key.
36. Concurrent adoption attempts are locked.

Dirty state

37. Dirty human checkout blocks adoption by default.
38. Dirty agent workspace blocks bless by default.
39. Discard warns before deleting dirty work.

Doctor/repair

40. agd doctor detects broken/missing metadata.
41. agd doctor detects stale AGD operation locks.
42. agd doctor detects incomplete Git states.
43. agd doctor detects moved human checkout path.
44. agd doctor --repair can fix safe drift.

Submodules/LFS

45. Repos with submodules are detected and warned.
46. Repos using Git LFS are detected and warned.
47. AGD does not silently create an obviously incomplete workspace.

⸻

38. First implementation slice: P0a

P0a should prove the core thesis before the full P0 surface is built.

Goal:

agent can commit unattended in an isolated workspace without using the human signing key, and the human can later adopt the work as one signed squash commit.

P0a includes:

manual proof script
Rust CLI scaffold
system Git wrapper
agd init
idempotent init
agd path
basic agd status
managed clone
project metadata
workspace marker
agent identity config
normal unsigned agent commits
explicit signing denial
denied signing attempt logging
push denial
basic squash agd bless
basic adoption lock
safety ref before adoption
dirty human checkout refusal for bless
dirty agent workspace refusal for bless

P0a excludes:

agd sync
agd branches/log/diff/files
agd bless --preserve
agd bless --merge
agd bless --continue
full conflict recovery
discard/reset-workspace
full doctor/repair
JSON output for every command
submodule/LFS completeness handling
PR mode
verify
handoff
workspace create/list

Conflict handling in P0a:

If squash adoption conflicts, AGD should leave a clear state, print exact next steps, and support a clean abort path. Full bless --continue behavior may wait until the broader P0 implementation.

Patch hash in P0a:

Do not include AGD-Patch-SHA256 until the stable serialization is precisely defined. Other provenance trailers may be included.

⸻

39. Implementation decisions for P0a

Binary name:

Use agd internally while building.

Platform:

Support macOS and Linux for P0a. Windows can wait.

Default workspace path:

~/.agd/workspaces/<repo-slug>-<project-short-id>/<workspace-id>

Default workspace ID:

default

Project ID:

Generate a stable project ID at init time and store it in project metadata and local markers.

Project lookup:

Use .git/agd/project.json in the human checkout and .agd/workspace.json in the agent workspace.

Repo-local config:

Do not create .agd.yaml by default.

Optional .agent symlink:

Defer.

Default target:

Use the current human checkout branch at agd init and store it as default_target. A later version may add an explicit --target option.

Protected branch commits in the agent workspace:

Warn by default, do not block normal Git commits.

Protected branch pushes:

Block by default through the push URL guardrail.

Squash authorship:

Human is author and committer by default. Agent provenance is recorded in AGD trailers.

Dirty state:

Refuse bless when the human checkout is dirty. Refuse bless when the selected agent workspace has uncommitted changes.

Safety refs:

Create refs/agd/safety/<operation-id> before mutating the human checkout during adoption.

Locking:

Use a simple project adoption lock for P0a bless. Broaden lock coverage later.

⸻

40. Phases

Phase 0 — Manual proof

Manual workflow:

independent clone
agent identity
signing disabled
explicit signing denied
push disabled
agent commits
human squash/sign

Deliverable:

scripts/manual-proof.sh

Goal: prove this solves the 1Password signing bottleneck.

Phase 1a — AGD P0a vertical slice

Implement:

Rust CLI
system Git wrapper
agd init
idempotent init
managed clone
project metadata
workspace marker
agent identity config
deny signer
push denied
agd path
basic agd status
basic squash bless
basic adoption lock
safety ref

Goal: make the narrow local workflow useful end to end.

Phase 1b — AGD full P0

Implement:

multi-workspace metadata schema
status/path/branches/log/diff/files
sync with precise semantics
preserve bless
signed merge
adoption locks/recovery
discard/reset
doctor/repair
JSON output
submodule/LFS detection

Phase 2 — PR and identity maturity

Add:

agd pr
project agent identity
session signing key
GitHub/GitLab integration
better conflict UX
branch summaries
commit message generation
agd verify
agd handoff
workspace create/list

Phase 3 — Convenience surfaces

Add:

TUI
IDE extension
shell prompt integration
harness plugins
Claude/Codex instructions or skills
agd shell
optional agd run

Phase 4 — Hardening / separate project

Add or integrate separately:

sandboxed agent checkout
APFS draft clone
VM/container backend
network policy
credential policy
audit transcript integration
Entire integration

⸻

41. Deferred questions

1. Final product/binary name.
2. Whether to create optional repo-local .agent symlink.
3. Whether minimal PR mode lands in late P0 or P1.
4. Whether branch summaries should be generated by AGD or left to agents.
5. Whether Entire/audit integration should be first-class later.
6. Exact stable patch-hash serialization for AGD-Patch-SHA256.
7. Whether handoff should be agd handoff or agd sync --include-uncommitted.
8. Whether workspace create/list should be P0 or P1.

⸻

42. One-line definition

AGD is a local Git delegation layer that gives agents their own commit identity and branch space, then lets humans review and sign/adopt the resulting work when ready.
