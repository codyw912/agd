**Goal:** Add the first alpha-release gate: CI for the standard local checks and a concise release checklist for tagging the first alpha.

**Architecture:** Keep CI independent from the Nix/devenv shell because this repository's flake has private inputs. Add a `fmt-check` recipe so CI can use the shared `just` entry point without mutating files. Document the alpha release checklist separately from the README so the README stays focused on users.

**Steps:**

1. Add `just fmt-check`.
2. Add GitHub Actions CI that runs format check, tests, and lint through `just`.
3. Add `docs/release-checklist.md` with alpha release gates and smoke tests.
4. Update README development commands to mention `fmt-check`.
5. Run `just fmt`, `just test`, and `just lint`.
