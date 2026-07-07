# Plan — p7b-kiln-production

**Phase:** p7b-kiln-production
**Planned:** 2026-07-04
**Change backend:** OpenSpec (`openspec/changes/`)
**Assessment:** `.kbd-orchestrator/phases/p7b-kiln-production/assessment.md`

---

## Ordering rationale

G1 and G3 are trivial and independent — they should run first to close the
two most impactful security gaps (epoch interruption + Cedar bypass) before
any other work. G2 is small-medium and makes G3 meaningful (Cedar fires on
real policies). G4/G5/G6 are independent P1/P2 work that can run in parallel
after the P0 triad is done.

```
Session 1:  c001 (epoch)           c003 (BGW identity) — parallel
Session 2:  c002 (Cedar DB)        — sequential after c001/c003
Session 3+: c004 (Fulcio)          c005 (DID HTTP)     c006 (integration) — parallel
```

---

## Change list

| # | Change ID | Title | Priority | Crates | Effort |
|---|---|---|---|---|---|
| 1 | **p7b-c001-epoch-interruption** | Wasmtime epoch timeout ticker | P0 | `fke-runtime` | Low |
| 2 | **p7b-c003-bgw-publisher-id** | BGW publisher `RlsContext` → Cedar fires | P0 | `fke-server` | Trivial |
| 3 | **p7b-c002-cedar-db-policies** | `DbKilnPolicySource` + migration 0009 | P0 | `fke-server`, migration | Low-Med |
| 4 | **p7b-c004-fulcio-chain** | Fulcio certificate chain validation | P1 | `fke-sign-cosign` | High |
| 5 | **p7b-c005-did-http-resolution** | DID HTTP resolver + TTL cache | P1 | `fke-sign-did` | Medium |
| 6 | **p7b-c006-integration-tests** | `testcontainers` integration harness | P2 | all `fke-store-*` | Medium |

---

## Constraint notes (from AGENTS.md)

- `#![forbid(unsafe_code)]` holds in all `fke-*` crates
- No `unwrap()`/`expect()` in library code — use `?` + thiserror/anyhow
- Files under 500 lines (`fke-runtime/src/lib.rs` is already 523 — c001 must not grow it further; split if needed)
- New crates go in `[workspace.dependencies]` first
- `fke-sign-*` and `fke-store-*` are adapter crates — do NOT import them from domain crates

---

## New workspace dependencies required

| Crate | Version | Used by | OQ |
|---|---|---|---|
| `sigstore` | `"0.14"` | c004 | OQ-P7B-1: audit API first |
| `testcontainers` | `"0.23"` | c006 | OQ-P7B-3: dev-dep only |
| `testcontainers-modules` | `"0.11"` | c006 | OQ-P7B-3: dev-dep only |

All other deps (`forge-identity`, `reqwest`, `serde_json`, `wiremock`) are already workspace deps.

---

## Phase gate

Phase is complete when:
- [ ] Epoch interruption wired and tested
- [ ] `DbKilnPolicySource` loaded from `flint_kiln.cedar_policies`
- [ ] BGW passes `Some(&publisher_rls)` to `EdgeRuntime::handle()`
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Recommended first action

```
/kbd-build p7b-c001-epoch-interruption
```

Start with epoch interruption — it's the safest first edit (3 lines in
`fke-runtime/src/lib.rs`, no new deps, no migrations), closes the most
obvious production gap, and unblocks running the full P0 triad in one session.
