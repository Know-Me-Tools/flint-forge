# p0-c001 — Workspace foundation

## Why
Establish the Flint Forge Cargo workspace and shared core so every later change has a
compiling base and the hexagonal dependency rule is enforced by the crate graph.

## What
- `[workspace]` with all non-pgrx members; `ext-flint-*` excluded (built via cargo-pgrx).
- Shared crates: `forge-domain` (pure types), `forge-identity` (JWT/RLS context, Option-3
  outbound), `forge-policy` (Cedar PEP trait).
- `/healthz` stubs in `fdb-gateway` and `fke-server`.
- CI: fmt + clippy::pedantic + `cargo check`.

## Contract
`cargo check` is green for the non-pgrx workspace. `forge-identity::RlsContext` and
`outbound_headers` exist with the signatures in the spec (§2.2, §3.5).

## Out of scope
Any adapter bodies (todo!() permitted), pgrx builds, the fabric RPC.
