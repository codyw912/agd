**Goal:** Finish package metadata needed for the first alpha release.

**Architecture:** Keep this release-hygiene only. Add the license files that match Cargo's dual-license expression, update the Nix package from the template placeholder name to `agd`, and make license verification part of the release checklist.

**Steps:**

1. Add `LICENSE-MIT` and `LICENSE-APACHE`.
2. Update `flake.nix` package metadata.
3. Update the release checklist with license verification.
4. Run format, test, lint, and release smoke gates.
