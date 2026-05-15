**Goal:** Record a repeated AGD PR push recovery observation from dogfooding.

**Architecture:** Keep this observation-only. Capture the SSH/1Password push failure during `agd pr --bless` and the successful `agd pr --continue` recovery without changing behavior yet.

**Steps:**

1. Add the observation to `docs/workflow-discovery.md`.
2. Run documentation-safe verification.
