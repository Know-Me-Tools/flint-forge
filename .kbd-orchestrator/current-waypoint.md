# Current Waypoint — Flint Forge

## Active Phase
**p16-production-remediation** — Remediate the 2026-07-12 production-readiness
audit to reach an honest, production-ready v1.0 (Supabase replacement for
agentic/AI-agent systems).

## Phase State
- Status: **executing**
- Changes planned: 9 (4 done, 0 in progress)
- OpenSpec changes: `p16-c001`, `p16-c002`, `p16-c004`, `p16-c005` archived (`openspec/changes/archive/`); `p16-c003`, `p16-c006` … `p16-c009` scaffolded (proposal.md + tasks.md each)
- Execution backend: **openspec**, self-executing (Claude Code CLI) via `/kbd-apply` — see `execution.md`

## Immediate Next Action
**Round 1 is fully COMPLETE: `p16-c001`, `p16-c002`, `p16-c004`, and `p16-c005`
are all DONE and archived.** c005 (auth hardening): replaced
`forge-identity`'s process-lifetime `OnceLock<JwkSet>`
(`crates/forge-identity/src/jwks.rs`) with an `ArcSwapOption`-backed TTL cache
(`FLINT_GATE_JWKS_TTL_SECS`, default 600s) plus a rate-limited
refetch-on-unknown-`kid` fast path (max once per 5s) for unplanned rotations;
made `FLINT_GATE_AUDIENCE` mandatory by default (`FLINT_GATE_MODE=production`
is now the default — `development` is the explicit, documented opt-out for
local iteration), matching every other default-flip already done in this
phase. Confirmed safe: no `docker-compose*.yml`/Helm file configures
`FLINT_GATE_*` today, so nothing currently deployed relied on the old lenient
default. Added 4 integration tests
(`crates/forge-identity/tests/{jwks_rotation,jwks_refetch_rate_limit,
audience_missing_fails_closed,audience_wrong_rejected}.rs`) against a real
`wiremock` JWKS server with genuine `ring`-generated ES256 keypairs — one test
per file so each gets its own process and never shares the crate's
process-global JWKS-cache statics or `FLINT_GATE_*` env vars with another
test. `cargo test --workspace` green; `cargo clippy` clean for everything
c005 touched (see `progress.json`'s `p16-c005` entry for a caveat about an
unrelated, separately-tracked in-flight task currently blocking the
whole-workspace clippy gate in `fke-sign-cosign`).

Run next:

```
/kbd-apply p16-c003-kiln-sandbox-authz
```

or, in parallel:

```
/kbd-apply p16-c006-config-truth-tracker-reconcile
```

Round 2: `p16-c003-kiln-sandbox-authz` (depends on c002, done — touches the
same `fke-server`/`fke-runtime` files) and `p16-c006` (depends on c001,
done) may run in parallel per the plan.
Then Round 3: `p16-c007`, `p16-c008` (the latter needs human/operator
involvement for credentials and backup drills).
Then Round 4: `p16-c009`.

Full ordering rationale in `.kbd-orchestrator/phases/p16-production-remediation/plan.md`;
dispatch contract in `.kbd-orchestrator/phases/p16-production-remediation/execution.md`.

## Why This Phase
Phase `p15-v1.0-production-readiness` was marked complete (pgrx stabilization,
migrations, CLI, e2e/perf CI, docs/Helm), and `v1.0.0` was tagged — but that tag
is an API/ABI contract freeze, not an operational-readiness claim. A
2026-07-12 audit (three independent subsystem reviews + a VGV enterprise
standards pass, both run through sycophancy-correction with a clean 0.0 score)
found three security-critical defects on live request paths that none of
p15's goals covered.

## P0 Blockers (must close before any production claim)
- REST/RPC tenant-isolation bypass (`p16-c001`).
- Kiln executes unsigned WASM (`p16-c002`).
- Kiln capability sandbox + authz not enforced (`p16-c003`).
- Realtime subscriptions deliver nothing by default (`p16-c004`).

## Verification Baseline
- `cargo check --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- Each P0 change ships with its own gate-defining integration test (see
  `assessment.md` / `plan.md` for the exact gate per change) — none of these
  exist yet.

## Effort Estimate
P0-only (minimum honest bar for a v1.0 claim): ~2–3 weeks. Full 9-change
remediation (P0–P3): ~4–6 weeks. Every P0 fix has a correct in-tree precedent
(GraphQL's RLS pattern, the written-but-unwired signer crates, the working
`listen` adapter) — this is wiring + tests, not new subsystems.
