# Workspace Discoverability

AGD keeps the human checkout and agent workspace separate. That separation is
part of the safety model: the agent gets its own clone, identity, branch space,
and push guardrails.

The current tradeoff is that managed workspaces live under AGD home, so they are
less obvious from the project directory.

## Current Entry Points

Use `agd shell` for interactive work:

```bash
agd shell
```

Use `agd path` when a script or manual shell command needs the workspace path:

```bash
cd "$(agd path)"
```

Use `agd workspace list` to see all recorded workspaces:

```bash
agd workspace list
```

Commands such as `path`, `status`, `review`, `sync`, `handoff`, `shell`, and
cleanup commands can also infer the current workspace when they run from inside
an agent workspace.

## Path Model

AGD currently stores:

- project metadata in AGD home
- a human checkout marker under `.git/agd/`
- managed workspace clones under AGD home
- a workspace marker inside each managed workspace

AGD does not currently create project-root `.agd/` files. That keeps AGD local
to the developer and avoids making the remote project AGD-aware.

## Symlink Direction

An ignored project-root symlink could make the default workspace easier to find:

```text
.agd/default -> ~/.agd/workspaces/<project>/default
```

This is attractive because it keeps the external workspace storage model while
making `cd .agd/default` possible from the human checkout.

Before implementing it, AGD should answer:

- should `.agd/` always be ignored, and who owns that ignore rule?
- how should `agd doctor --repair` recreate or remove a broken symlink?
- what should happen on platforms where symlink creation is restricted?
- should named workspaces appear as `.agd/<workspace-id>`?
- will editors or language servers index both the human checkout and symlinked
  workspace and produce duplicate diagnostics?
- could tools inside the symlinked workspace accidentally discover the human
  checkout as an outer project?
- should the symlink be created by default or by an explicit command?

## Current Recommendation

Use `agd shell` as the default interactive entry point. Use `agd path` for
scripts. Treat project-root symlinks as a design track until dogfooding proves
the cleanup, repair, editor, and cross-platform behavior.

If AGD adds symlinks later, the likely shape is:

- opt-in first
- ignored by default
- repairable by `agd doctor --repair`
- named consistently with workspace ids
- removable without affecting the real managed workspace
