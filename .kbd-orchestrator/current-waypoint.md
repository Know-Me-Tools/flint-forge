# Current Waypoint — Flint Forge

## Active Phase
**p16-production-remediation** — Remediate the 2026-07-12 production-readiness
audit to reach an honest, production-ready v1.0 (Supabase replacement for
agentic/AI-agent systems).

## Phase State
- Status: **executing**
- Changes planned: 9 (5 done, 0 in progress)
- OpenSpec changes: `p16-c001`, `p16-c002`, `p16-c003`, `p16-c004`, `p16-c005` archived (`openspec/changes/archive/`); `p16-c006` … `p16-c009` scaffolded (proposal.md + tasks.md each)
- Execution backend: **openspec**, self-executing (Claude Code CLI) via `/kbd-apply` — see `execution.md`

## Immediate Next Action
**Round 1 AND `p16-c003` (Round 2) are all DONE and archived** —
`p16-c001`, `p16-c002`, `p16-c003`, `p16-c004`, `p16-c005`. c003 (Kiln
sandbox + authz): fixed the literal `check_capabilities(granted, granted)`
no-op — `granted` is now computed independently per capability via a new
per-capability Cedar action (`kiln:capability:<name>`, distinct from
`kiln:invoke`); added mandatory bearer auth to `/functions/v1/<name>`
(missing/invalid → 401 before ever reaching the runtime, making
`caller = None` genuinely unreachable from that HTTP path) and a new
`require_admin()` gate to `/admin/functions` (401 unauthenticated, 403
non-`service_role`) — previously gated only by a compile-time feature flag.
Deliberately deferred wiring the `flint:host@0.1.0` WIT host functions
(Db/Llm/Kv/Identity/Secrets) into the linker — investigated first: no
component anywhere in this repo imports them yet, `db`/`llm`/`secrets` need
live backing clients this crate has no access to, and none of the five can
be verified end-to-end without the still-unavailable `cargo-component`
toolchain — spawned as its own follow-up (`task_22a1dcc7`) rather than
shipped unverified, matching c002's SCT/OIDC-allowlist precedent. See
`progress.json`'s `p16-c003` entry for the full account, including a note
on independent convergence with a concurrently-running background session
that reached the same conclusions.

Run next:

```
/kbd-apply p16-c006-config-truth-tracker-reconcile
```

Round 2 is now fully complete (`p16-c003` done here; `p16-c006` depends on
c001, done — safe to start next).
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
