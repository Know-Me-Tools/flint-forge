# Reflection — anon_and_service_role_keys

Date: 2026-07-09
Phase status: complete
Goal completion: 5/5 changes complete (100%)

## Goal Achievement

| Goal | Result | Notes |
| --- | --- | --- |
| Forge keygen and token claims | MET | `forge keygen init` / `rotate` now emits Supabase-style anon and service-role material, and `forge token mint` accepts role, principal type, agent/workflow, scope, tenant/session, and extra JSON claims. |
| Forge auth SQL role model | MET | `ext-flint-auth` now defines `anon`, `authenticated`, `agent`, `service_role`, and `authenticator` behavior with helper functions and `auth.api_keys`. |
| Gate API-key roles and trusted headers | MET | Gate persists role/principal metadata, maps key records into identity kinds, rejects browser-presented secret keys, strips spoofed inbound Flint headers, and injects trusted upstream headers. |
| Realtime principal metadata propagation | MET | Realtime verified claims now preserve role, principal type, agent id, workflow id, and scope. |
| Cross-project configuration docs | MET | Key-contract docs were added to Forge, Gate, and Realtime Fabric. |

## Delivered Changes

- Forge CLI can initialize and rotate project key material and mint richer JWTs for `anon`, `authenticated`, `agent`, and `service_role`.
- Forge auth SQL now includes the agent-aware role model, principal helper functions, and an `auth.api_keys` compatibility table.
- Gate API-key validation now carries role metadata and forwards only trusted Flint identity headers to upstream services.
- Realtime Fabric now keeps Flint role and principal metadata after JWT verification.
- Documentation now describes the shared key contract across `flint-forge`, `flint-gate`, and `flint-realtime-fabric`.

## Verification

- `cargo test -p forge-cli` passed.
- `cargo check -p flint-gate-core` passed.
- `cargo test -p flint-gate-core api_key --lib` passed.
- `cargo check -p frf-ports -p frf-identity-ory` passed.
- `cargo check -p frf-app -p frf-gateway` passed.
- `cargo clippy -p forge-cli -- -D warnings` passed.
- `cargo clippy -p frf-ports -p frf-identity-ory -- -D warnings` passed.
- `cargo run -p forge-cli -- keygen init --project smoke --env test --format json --quiet` emitted anon and service-role JWTs.

`cargo clippy -p flint-gate-core --lib -- -D warnings` remains blocked by an unrelated pre-existing `clippy::result_large_err` in `crates/flint-gate-core/src/admin/mod.rs`.

## Artifact Quality Summary

| Metric | Value |
| --- | --- |
| Changes with QA | 0/5 |
| First-pass pass rate | N/A |
| Changes requiring refinement | 0 recorded |
| Total refinement iterations | 0 recorded |

No `.refiner/artifacts/<change-id>/refinement_log.md` files were present for this phase, so artifact-refiner pass rates and constraint violations could not be computed.

### Recurring Constraint Violations

- None recorded. Artifact-refiner logs were absent.

## Technical Debt Introduced

- The phase exceeded the 3-wait budget: `progress.json` records `total_waits: 6` after cross-repo check, test, clippy, and smoke iterations.
- Gate full clippy is still not green because of an unrelated existing lint in `admin/mod.rs`.
- Native KBD changes were tracked directly in `progress.json`; no per-change archive directories were created.
- Production key rotation remains a contract-level implementation: the CLI emits next-secret material, but coordinated multi-secret validation and expiry windows should be hardened in the serving components.

## Lessons Captured

- Cross-repo identity contracts need one shared set of claim names. Preserving `role`, `principal_type`, `agent_id`, `workflow_id`, and `scope` end to end avoided adapter-specific interpretations.
- Trusted identity headers should be constructed after client header stripping; otherwise upstream services cannot distinguish verified metadata from user input.
- Supabase-style `service_role` support needs explicit browser-client rejection, not only documentation that the key is server-only.
- The compile/test wait budget should be spent only after batching all sibling-repo edits. Clippy iterations across three repos can easily exceed the budget if not scoped aggressively.

## Recommended Next Phase

Focus the next phase on production hardening for the key lifecycle:

- Add coordinated signing-secret rotation with current/next/previous validation windows in Gate and Realtime.
- Add an integration smoke that generates keys in Forge, validates through Gate, and preserves metadata into Realtime.
- Resolve the existing Gate clippy blocker so future cross-repo phases can use full `-D warnings` as a clean gate.
- Add artifact-refiner logging or explicit QA skips for native KBD changes so reflection can compute quality metrics.
