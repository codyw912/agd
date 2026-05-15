# Release Checklist

Use this checklist before tagging an alpha release.

## Version

- Confirm `Cargo.toml` has the intended release version.
- Confirm `flake.nix` package metadata uses the same version.
- Decide whether the tag is a final alpha version such as `v0.1.0` or a prerelease such as `v0.1.0-alpha.1`.

## Verification

- Run `just fmt-check`.
- Run `just test`.
- Run `just lint`.
- Confirm GitHub Actions passes on the release PR.

## Smoke Test

Run the core flow in a disposable repository or fresh local clone:

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
