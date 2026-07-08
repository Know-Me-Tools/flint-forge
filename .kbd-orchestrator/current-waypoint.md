# Current Waypoint — Flint Forge

## Active Phase
**p15-v1.0-production-readiness** — v1.0 Production Readiness Gap Closure

## Phase State
- Status: **in_progress**
- Changes planned: 5 (0 done)
- Active: `p15-c001`, `p15-c002`

## Immediate Next Action
1. Upgrade pgrx extensions to 0.18.1 + pg18 (`p15-c001`).
2. Renumber colliding migrations and add CI migrate test (`p15-c002`).
3. Run workspace checks after each coherent slice.
4. Update progress in `.kbd-orchestrator/phases/p15-v1.0-production-readiness/progress.json`.

## Why This Phase
The core server plane (Quarry + Kiln) compiles and passes 470+ unit tests, but
the pgrx extension suite (Anvil) does not build, migrations have sequence
collisions, the operator CLI is a stub, and end-to-end validation is missing.
This phase closes those gaps so v1.0 is production-credible.

## P0 Blockers
- Anvil pgrx extensions do not compile.
- Migration sequence collisions prevent clean `sqlx migrate run`.

## Verification Baseline
- `cargo check --workspace` passes.
- `cargo test --workspace --lib --bins` passes.
- `cargo clippy --workspace -- -D warnings` passes.
