**Goal:** Make the alpha release smoke proof a first-class project command.

**Architecture:** Reuse `scripts/manual-proof.sh` instead of adding a second smoke path. The script already creates a disposable repository and exercises the local trust boundary, so extend it with doctor, status, review, sync, and verify checks, then expose it through `just release-smoke`.

**Steps:**

1. Add a `release-smoke` just recipe.
2. Extend the manual proof script with release-relevant command checks.
3. Update the release checklist to use the recipe before any hosted PR smoke.
4. Run the release smoke recipe and the normal release gate.
