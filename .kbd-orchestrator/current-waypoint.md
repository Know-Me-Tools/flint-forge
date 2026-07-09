# Current Waypoint — Flint Forge

## Active Phase
**anon_and_service_role_keys** — Supabase-style anon/service-role key support

## Phase State
- Status: **completed**
- Changes planned: 5 (5 done)
- Active: none

## Immediate Next Action
1. Run `/kbd-reflect anon_and_service_role_keys` when ready.
2. Review the wait-budget overrun recorded in `progress.json`.

## Why This Phase
This phase adds the shared Flint key contract: client-safe anon keys,
server-only service-role keys, and agent-aware claims that flow across Forge,
Gate, and Realtime.

## P0 Blockers
- None remaining for this implementation slice.

## Verification
- `cargo test -p forge-cli` passes.
- `cargo clippy -p forge-cli -- -D warnings` passes.
- `cargo check -p flint-gate-core` passes.
- `cargo test -p flint-gate-core api_key --lib` passes.
- `cargo check -p frf-ports -p frf-identity-ory` passes.
- `cargo check -p frf-app -p frf-gateway` passes.
- `cargo clippy -p frf-ports -p frf-identity-ory -- -D warnings` passes.
- `cargo run -p forge-cli -- keygen init --project smoke --env test --format json --quiet` emits anon and service-role JWTs.

## Carry-Forward From p15
- Artifact-refiner logs were not present for p15 changes.
- The p15 progress ledger records 5 waits, above the 3-wait budget.
- k6 baselines are local Colima baselines and should be re-run against production-like staging.
