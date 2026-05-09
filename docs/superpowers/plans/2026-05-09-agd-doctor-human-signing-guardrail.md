# Doctor Human Signing Guardrail Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `agd doctor` detect when AGD's deny signer has leaked into the human checkout signing config.

**Architecture:** Keep the human checkout signing policy flexible: doctor should not require signing to be enabled or disabled. It should only fail when the human checkout's `gpg.program` equals the managed workspace deny signer, because that means AGD's agent-only signing guardrail has contaminated the human checkout.

**Tech Stack:** Rust, system Git config checks, assert_cmd integration tests, `just` command recipes.

---

### Task 1: Detect Human Deny Signer Contamination

**Files:**
- Modify: `src/doctor.rs`
- Modify: `tests/p0a.rs`

- [x] **Step 1: Write the failing test**

Add `doctor_detects_human_checkout_using_agd_deny_signer` to `tests/p0a.rs`. Initialize AGD, read the workspace `gpg.program`, set the human checkout `gpg.program` to the same value, run `agd doctor`, and assert it fails with `human signing` and `AGD deny signer`.

- [x] **Step 2: Run test to verify it fails**

Run: `just dev cargo test doctor_detects_human_checkout_using_agd_deny_signer`

Expected: FAIL because current doctor does not inspect human checkout signing contamination.

- [x] **Step 3: Implement minimal check**

In `src/doctor.rs`, add `human_signing_check(project, &workspace.path)` and call it after the workspace guardrail checks. It should read `gpg.program` from both the human checkout and workspace. If the human value is absent, empty, or different, return ok. If equal, return fail with detail indicating the human checkout uses the AGD deny signer.

- [x] **Step 4: Run focused test to verify it passes**

Run: `just dev cargo test doctor_detects_human_checkout_using_agd_deny_signer`

Expected: PASS.

- [x] **Step 5: Run full verification**

Run:

```bash
just fmt
just dev cargo fmt --all -- --check
just test
just lint
just dev-shell 'bash scripts/manual-proof.sh'
```

Expected: all commands exit 0.

- [x] **Step 6: Commit**

```bash
git add docs/superpowers/plans/2026-05-09-agd-doctor-human-signing-guardrail.md src/doctor.rs tests/p0a.rs
git commit -m "feat: detect human checkout deny signer drift"
```
