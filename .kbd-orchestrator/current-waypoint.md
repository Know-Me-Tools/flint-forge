# Current Waypoint — Flint Forge

## Active Phase
**p16-production-remediation** — Remediate the 2026-07-12 production-readiness
audit to reach an honest, production-ready v1.0 (Supabase replacement for
agentic/AI-agent systems).

## Phase State
- Status: **executing**
- Changes planned: 9 (7 done, 0 in progress)
- OpenSpec changes: `p16-c001` … `p16-c007` all archived (`openspec/changes/archive/`); `p16-c008`, `p16-c009` scaffolded (proposal.md + tasks.md each)
- Execution backend: **openspec**, self-executing (Claude Code CLI) via `/kbd-apply` — see `execution.md`

## Immediate Next Action
**Rounds 1 AND 2 are fully DONE, and Round 3's `p16-c007` is too** —
`p16-c001` through `p16-c007` all archived. c007 (500-line file-size
compliance): re-measured line counts (18 files now over the limit, up from
17 at the original audit), split 17 into directory modules as pure
mechanical refactors — zero behavior change, `cargo check`/`clippy`/`test`
all green workspace-wide (76/76 test-result summaries `ok`). Executed via
~15 parallel subagents, since `mod foo;` resolves to either `foo.rs` or
`foo/mod.rs` automatically — most splits touch zero shared parent files, so
cross-agent conflicts were largely eliminated by construction. One real
conflict did surface: two independent concurrent sessions both split the
largest file (`routes/htmx/renderers.rs`, 1267 lines), and the copy that
landed in the shared working tree had a genuine visibility bug breaking the
whole `fdb-gateway` crate — resolved by finding a second, independently
verified split sitting in an isolated agent worktree and substituting it in
wholesale. Deliberately NOT split: `crates/ext-flint-vault/src/lib.rs` (513
lines) — the pgrx-based envelope-encrypted secret store / KMS-wrapped-DEK
extension, the single most security-critical crate in the repo — because
`cargo-pgrx` requires `$PGRX_HOME` (needs network access to build a local
Postgres via `cargo pgrx init`), confirmed genuinely unavailable here, and
splitting security-critical pgrx code with zero ability to even
compile-check it was judged too high a risk. Left as tracked, documented
open debt. See `progress.json`'s `p16-c007` entry for the full account.

Run next:

```
/kbd-apply p16-c008-production-operations
```

Round 3's last change, `p16-c008` (production operations), needs
human/operator involvement for credentials and backup drills — not
something achievable unattended in this environment.
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
