# Plan — p6b-kiln-hardening

**Phase:** p6b-kiln-hardening  
**Planned:** 2026-07-04  
**Change backend:** OpenSpec (`openspec/changes/`)  
**Assessment:** `.kbd-orchestrator/phases/p6b-kiln-hardening/assessment.md`

---

## Ordering rationale

`c001 → c002` must run in sequence (BGW calls the Cedar gate). All other
changes are independent and can execute in parallel. The P0 triad (`c001 →
c002 → c003`) is the MVP critical path. P1/P2 changes (`c004–c008`) round
out the security and storage layers.

```
Session 1:  c001 (Cedar gate) → c002 (Kiln BGW)
Parallel:   c003 (WIT bindings)   c004 (sign-did)   c005 (sign-cosign)
Parallel:   c006 (store-oci)      c007 (store-ipfs)  c008 (store-s3)
```

---

## Change list

| # | Change ID | Title | Priority | Crates touched | Est. effort |
|---|---|---|---|---|---|
| 1 | **p6b-c001-cedar-gate** | Cedar `Pep` injection into `EdgeRuntime` | P0 | `fke-runtime`, `forge-policy`, `fke-server` | Medium |
| 2 | **p6b-c002-kiln-bgw** | Background worker draining `webhook_outbox` | P0 | `fke-server` | Low-Med |
| 3 | **p6b-c003-wit-bindings** | WIT contract + real `wasi:http` dispatch | P0 | `fke-domain` (WIT), `fke-runtime`, `Cargo.toml` | High |
| 4 | **p6b-c004-sign-did** | Ed25519 / `did:prometheus` verifier | P1 | `fke-sign-did` | Medium |
| 5 | **p6b-c005-sign-cosign** | Sigstore/Cosign verifier (Rekor + ECDSA P-256) | P1 | `fke-sign-cosign` | High |
| 6 | **p6b-c006-store-oci** | OCI registry artifact store | P1 | `fke-store-oci` | Medium |
| 7 | **p6b-c007-store-ipfs** | Kubo HTTP API artifact store | P2 | `fke-store-ipfs` | Low |
| 8 | **p6b-c008-store-s3** | S3/R2 artifact store via `object_store` | P2 | `fke-store-s3` | Low-Med |

---

## Constraint notes (from AGENTS.md)

- All new crates go in `[workspace.dependencies]` first — check before adding.
- `#![forbid(unsafe_code)]` holds in all `fke-*` crates.
- No `unwrap()`/`expect()` in library crates — use `thiserror` or `anyhow::Result`.
- Files must stay under 500 lines — split large impls into sub-modules.
- `fke-sign-*` and `fke-store-*` are adapter crates — only `fke-server` may compose them; `fke-domain`/`fke-ports` must not import them.

---

## New workspace dependencies required

| Crate | Version | Used by |
|---|---|---|
| `wasmtime-wasi-http` | `"26"` | c003 |
| `ed25519-dalek` | `"2"` | c004 |
| `sha2` | `"0.10"` | c003, c004, c005, c006, c008 |
| `chrono` | `"0.4"` | c004, c005 |
| `base64` | `"0.22"` | c004, c005 |
| `p256` | `"0.13"` | c005 |
| `oci-client` | `"0.14"` | c006 |
| `object_store` | `"0.11"` | c008 |

`reqwest`, `wiremock`, `tokio`, `anyhow` are already workspace deps.

---

## Phase gate

Phase is complete when all P0 changes are done AND either `fke-sign-did` or
`fke-sign-cosign` is non-`todo!()` AND either `fke-store-oci` or `fke-store-s3`
is non-`todo!()`, plus workspace clippy and tests are green.

---

## Recommended first action

```
/kbd-build p6b-c001-cedar-gate
```

Start with the Cedar gate — it is the safest first step (pure Rust, well-established
pattern in the codebase), unblocks the BGW, and has no new external dependencies
beyond `forge-policy` which is already in the workspace.
