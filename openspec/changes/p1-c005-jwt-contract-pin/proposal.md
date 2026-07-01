# p1-c005 — JWT contract pin

## Why

The claim shape emitted by flint-gate is the load-bearing contract for all RLS policies, the `auth.*` GUC functions, and Phase 2's reflection engine. Without a pinned contract derived from actual flint-gate source code, every downstream consumer risks a silent mismatch.

## What

`docs/contracts/jwt-contract.md` — already written this session from flint-gate source code. Contains:
- §1: Inbound token — claim extraction rules from `jwt_verify.rs`
- §2: Outbound minted token — claim shape from `jwt_mint.rs`, algorithm options
- §3: `SET LOCAL` GUC propagation — what Postgres actually sees
- §4: Service-identity token format — `role: "service_role"` in `additional_claims`
- §5: Phase 2+ claim extensions
- §6: Security constraints
- §7: Integration points for `ext-flint-auth`

## Contract

`docs/contracts/jwt-contract.md` exists, is accurate against the flint-gate source, and documents the `role` claim injection requirement (must be in `additional_claims` per route hook — not automatically included).

## Status

**COMPLETE.** Written during the assessment phase of this session (2026-06-30). No further code changes required.

## Out of scope

The flint-gate configuration for deploying with the correct `additional_claims` — that is an ops concern for the deployment phase.

## Reference

- `docs/contracts/jwt-contract.md` (complete)
- `flint-gate/crates/flint-gate-core/src/auth/jwt_mint.rs`
- `flint-gate/crates/flint-gate-core/src/auth/jwt_verify.rs`
- `flint-gate/crates/flint-gate-core/src/auth/identity.rs`
