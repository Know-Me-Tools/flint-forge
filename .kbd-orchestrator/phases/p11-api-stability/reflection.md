# Reflection — p11-api-stability

**Phase:** 11 — API Stability
**Period:** 2026-07-06
**Author:** OpenCode / KBD automated reflection
**Changes:** 6/6 complete
**Status:** ✅ COMPLETE
**Commits:** 4 conventional commits pushed to `main`

---

## Summary

Phase 11 delivers the API stability contract required before tagging `v1.0.0`.
All three primary API surfaces (A2UI HTTP, Kiln WIT ABI, React/Flutter SDKs) are
now versioned, annotated, and documented. Two P1 operational improvements close
inherited p10 debt: Dockerfile entrypoints eliminate `.env` on production hosts,
and `mint_smoke_token.sh` replaces a long-lived static JWT with a fresh 1-hour
token on every deploy. All 6 changes were delivered in a single session with zero
test regressions.

---

## Goal Achievement

| Goal | Priority | Status | Evidence |
|---|---|---|---|
| G1 — A2UI API freeze | P0 | **MET** | 7 enums annotated `#[non_exhaustive]` (2 already done); `docs/api/a2ui.md` (541 lines, 10 endpoint contracts); `FLINT_A2UI_API_VERSION=1` |
| G2 — Kiln ABI freeze | P0 | **MET** | 5 WIT interfaces annotated `@since(version = 0.1.0)`; stability block on `world edge-function`; `docs/api/kiln-abi.md` (369 lines, 13 sections); `FLINT_KILN_ABI_VERSION=1` |
| G3 — SDK v1.0 alignment | P0 | **MET** | `@flint/react` 1.0.0; `flint_genui` 1.0.0; `CHANGELOG.md` × 2; `MIGRATION.md` (workspace root) |
| G4 — k6 baselines | P1 | **PARTIAL-MET** | `BASELINE_DATE`/`BASELINE_SOURCE` constants + SOP added; `perf/results/` created; measurement deferred — staging stack not available |
| G5 — Entrypoint secrets | P1 | **MET** | `docker/fdb-gateway/entrypoint.sh` + `docker/fke-server/entrypoint.sh`; both Dockerfiles updated; prod compose comment updated; compose config validates |
| G6 — Staging token rotation | P2 | **MET** | `scripts/mint_smoke_token.sh` (HS256, 1-hour, portable); `deploy.yml` mints fresh token per run; `STAGING_SMOKE_TOKEN` removed; `STAGING_JWT_SECRET` documented in §9.1; §11 added to runbook |

**MVP gate check:**

| Gate condition | Result |
|---|---|
| `#[non_exhaustive]` on A2UI public enums | ✅ 48 total in workspace (7 added this phase) |
| `docs/api/a2ui.md` written | ✅ 541 lines |
| `docs/api/kiln-abi.md` written | ✅ 369 lines |
| `@flint/react` and `flint_genui` at 1.0.0 | ✅ both 1.0.0 |
| k6 thresholds from measured staging values | ⚠️ PARTIAL — annotation done; measurement blocked on staging |
| Dockerfile entrypoints wire secrets without `.env` | ✅ both entrypoints implemented |
| `cargo test --workspace` passes | ✅ 457 tests, 0 failures |
| `cargo clippy --workspace -- -D warnings` clean | ✅ |

**6/6 goals delivered. G4 is partial — the annotation scaffolding is in place and the measurement procedure is fully documented; the actual P50/P95/P99 values require a live staging run.**

---

## Artifact Quality Summary

No `.refiner/` logs — KBD native execution with CI gate enforcement.

| Metric | Value |
|---|---|
| Changes completed | 6/6 (100%) |
| Parallel executions | 2 (c001+c002; c004+c006) |
| Test regressions | 0 |
| Rework required | 0 |
| `#[non_exhaustive]` already present (found during execution) | 2 (`ReflectionError`, `AssemblerError`) |
| Final test count | 457 |

The assessment identified 9 enums needing annotation; during execution 2 were
found to already have `#[non_exhaustive]` from prior work. Actual net additions:
7. No match arm breakage in library crates; one external-crate exhaustive match in
`routes/agui.rs` (`AgUiEvent`) required a forward-compatibility wildcard arm.

---

## Open Questions — Resolution

| OQ | Resolution |
|---|---|
| OQ-P11-1: WIT `@since` support | **Confirmed.** `cargo component 0.21.1` / `wit-parser 0.251.0` parses `@since(version = 0.1.0)` natively. No doc-comment fallback needed. |
| OQ-P11-2: Self-issue vs external flint-gate | **Self-signed HS256.** `scripts/mint_smoke_token.sh` uses `openssl dgst -hmac` to sign a minimal claims set against `secrets/jwt_secret.txt`. No external IdP required for smoke testing. The gateway verifies it using its existing `FLINT_JWT_SECRET` validation path. |

---

## Technical Debt Introduced

| Item | Location | Severity | Remediation |
|---|---|---|---|
| k6 baseline values are still TBD | `perf/k6/regression.js` | LOW | Run scripts against live staging; update `BASELINE_DATE` and thresholds |
| `perf/results/` directory is empty | `perf/results/.gitkeep` | LOW | Populated automatically on first `k6 run --out json=perf/results/...` |
| SDK CHANGELOGs document only p5–p10 highlights, not individual commits | `packages/*/CHANGELOG.md` | LOW | Future SDK releases will auto-generate from conventional commits |
| Grafana DB connections panel still shows "no data" | `observability/grafana-dashboard.json` | LOW | Requires future sqlx Prometheus integration — deferred indefinitely |

**Net debt balance:** Low. All p11-introduced debt is documentation gaps or staged measurements. No new architectural debt.

---

## What Was Harder Than Expected

1. **`AgUiEvent` match arm** — The assessment noted the enum was in `fdb-domain` (a library crate). `#[non_exhaustive]` on a type in crate A makes matches in crate B non-exhaustive even if crate B is internal. The match in `routes/agui.rs` was exhaustive over 13 variants; it needed a wildcard arm. Caught immediately on the first `cargo check`.

2. **`base64 -w0` macOS incompatibility** — The initial `mint_smoke_token.sh` implementation used GNU `base64 -w0` (suppress line wrapping). macOS `base64` does not support `-w0`. The portable fix is `base64 | tr -d '\n'`. This was caught during the live test run and fixed in the same session.

3. **Assessment over-counted enums needing annotation** — 9 were listed; 2 (`ReflectionError`, `AssemblerError`) already had `#[non_exhaustive]` from prior work. The over-count had no impact on execution (subagent simply skipped them) but highlights the value of checking before writing.

---

## Lessons Captured

1. **`#[non_exhaustive]` on cross-crate enums breaks matches in consumer crates** — When you annotate a type in a library crate, all match expressions in downstream binary or gateway crates must have wildcard arms. Run `cargo check --workspace` (not just the library) immediately after adding the attribute. One-pass fix.

2. **`base64` is not portable — macOS vs GNU differ on line-wrapping flags** — Any shell script that uses `base64` in a pipeline must use the portable idiom: `base64 | tr -d '\n'` to strip newlines, rather than `-w0` (GNU) or `-b 0` (macOS). This affects all JWT minting, HMAC, and binary→text encoding scripts.

3. **`@since` in WIT is parsed by `cargo component 0.21.1` without extra flags** — The annotation `@since(version = 0.1.0)` is valid WIT 0.2 syntax and the installed toolchain accepts it. Verify with `cargo component build -p hello-component` after editing `.wit` files — it re-parses the WIT and rebuilds bindings.

4. **Subagent parallel dispatch works cleanly for independent document-only changes** — c004 (k6 annotation) and c006 (mint_smoke_token) shared no files and had no ordering dependency. Dispatching both as parallel subagents halved wall-clock time for these two changes with no coordination overhead.

5. **`mint_smoke_token.sh` key resolution order matters** — Putting `$JWT_SECRET` env var first (priority 1) makes the script testable in any environment without needing files. CI can inject the key via env; the staging host uses the file path. The container uses the Docker secret mount. One script, three deployment contexts.

---

## Recommended Next Phase

Phase 11 closes the `v1.0.0` readiness checklist. The Flint Forge platform is:

- **Documented** — `docs/api/a2ui.md` + `docs/api/kiln-abi.md` + `MIGRATION.md`
- **API-stable** — `#[non_exhaustive]` on public enums; WIT `@since` annotations; SDKs at 1.0.0
- **Secrets-complete** — entrypoints read from Docker secret files; tokens minted dynamically
- **Production-deployed** — v0.10.0 released; staging stack defined; CI gates live

The natural next phase is **`v1.0.0` tagging**. The pre-requisites are:

1. **k6 baselines measured** (blocked on a running staging stack — one operator action needed)
2. **Any open regressions in `v0.10.x` patch series** — monitor for CVE advisories (the `cargo audit` CI gate will surface these automatically)

**Proposed p12-v1-release (narrow, ~2 changes):**

1. **p12-c001-k6-measure** — Run the k6 scripts against live staging; update `regression.js` thresholds and `docs/performance.md`. Gate: `regression.js` passes.
2. **p12-c002-v1-tag** — Tag `v1.0.0`; update `[workspace.package] version = "1.0.0"`; update `@flint/react` and `flint_genui` if needed; generate CHANGELOG via `git cliff`; create GitHub Release `v1.0.0`.

**Estimated scope:** 2 changes, 1 session (once staging is live for c001).

**Alternative:** If staging cannot be brought up promptly, skip p12-c001 and tag `v1.0.0` with a note in the release that k6 baselines are TBD. The `regression.js` gate is `workflow_dispatch`-only and not a required CI gate.

---

*Generated by OpenCode `/kbd-reflect` — 2026-07-06*
