# Release Checklist

Use this checklist before tagging an alpha release.

## Version

- Confirm `Cargo.toml` has the intended release version.
- Confirm `flake.nix` package metadata uses the same version.
- Confirm `CHANGELOG.md` has a release entry for the same version.
- Confirm the release tag will be `v0.1.0`.
- Confirm license files match the package license metadata.

## Verification

- Run `just fmt-check`.
- Run `just test`.
- Run `just lint`.
- Run `just package-check`.
- Confirm GitHub Actions passes on the release PR.

## Smoke Test

Run the local release smoke proof:

```bash
just release-smoke
```

This creates a disposable repository and checks init, doctor, status, agent
identity, signing denial, push denial, review commands, sync, bless, and
verification.

Then run the hosted PR flow in a disposable repository or fresh local clone:

```bash
agd init
agd doctor
cd "$(agd path)"
git switch -c agent/release-smoke
git commit --allow-empty -m "Smoke test AGD release"
agd status
agd sync --rebase
agd pr --bless agent/release-smoke
```

Confirm the smoke test opens a human-owned pull request and that the PR body includes AGD provenance.

## Release Notes

- Confirm `CHANGELOG.md` describes the release scope and known limitations.
- Summarize the supported alpha workflow.
- Call out that AGD separates Git authority but does not sandbox execution.
- List known limitations and recovery commands that changed in the release.
- Include the install command or download path for the release artifact.

## Tagging

- Create the release tag from `main` after the release PR is merged.
- Push the tag only after CI passes on `main`.

```bash
git switch main
git pull --ff-only
git status --short
git tag -a v0.1.0 -m "AGD v0.1.0"
git push origin v0.1.0
```

After the tag is pushed, verify installation from the tag:

```bash
cargo install --git https://github.com/codyw912/agd.git --tag v0.1.0 --locked
agd --help
```
