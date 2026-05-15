**Goal:** Make Cargo packaging validation part of the alpha release gate.

**Architecture:** Add a shared `just package-check` recipe so local release checks and CI use the same command entry point. Keep the check Cargo-native and independent from Nix so it validates the crate artifact people can install with Cargo.

**Steps:**

1. Add `just package-check`.
2. Run `package-check` in CI.
3. Add package validation to the release checklist and README development commands.
4. Run package, smoke, format, test, and lint gates.
