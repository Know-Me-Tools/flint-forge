# p0-c002 — Tasks &amp; Status

## GATE: GREEN (option (c) — full data-plane image, pg_graphql deferred to Phase 3)

Built and verified end-to-end on a live container (`flint-forge-pg:18`, 674 MB):
```
boot assertion: OK     wal_level=logical     shared_preload_libraries=pg_net
extensions: flint_auth 0.1.0 · flint_hooks 0.1.0 · flint_llm 0.1.0 ·
            pg_net 0.20.3 · pgcrypto 1.4 · vector 0.8.3
functional: public.llm_version() resolves (pgrx) · net.http_post/http_get present (source) ·
            auth.role() → 'anon' with no JWT
```

## Done
- [x] `Dockerfile.baseline` — verified subset (pgvector + pgcrypto + Flint SQL schemas).
- [x] `Dockerfile` (full, option c) — multi-stage:
  - [x] **anvil**: `flint_llm` compiled via cargo-pgrx 0.18.1 / pg18 (rust:1.96 base).
  - [x] **pgnet**: `pg_net` 0.20.3 built from source; `libcurl4` added to runtime.
  - [x] runtime: pgvector (apt) + pgcrypto (base) + SQL-only `flint_auth`/`flint_hooks`.
- [x] Config: `wal_level=logical`, `shared_preload_libraries=pg_net`.
- [x] First-boot creates all extensions; boot assertion passes.
- [x] Corrected `flint_llm` crate: self-contained manifest, pg18, `pgrx_embed` bin,
      `.control`, `extension_sql_file!("../sql/...")`.

## Resolved unknowns (was spec §8)
- [x] pgrx + PG18 → `=0.18.1` (built & loaded). flint_auth/hooks are SQL-only (no pgrx).
- [x] pg_net not in apt → source build (verified). libcurl4 runtime dep captured.

## Deferred to Phase 3 (by decision)
- [ ] **pg_graphql** — no released PG18 build (supabase/pg_graphql#614). Not on the critical
      path for Phases 1–2 (REST + Anvil). Revisit at Phase 3: build from a pinned master SHA,
      or run the GraphQL instance on PG17 until v1.5.12. `GraphQlExecutor` port is unaffected.

## Deferred to Phase 4
- [ ] `flint_llm` background worker → then add `flint_llm` to `shared_preload_libraries`.
      (Compiled and installed now; no BGW to preload yet, so left out of preload.)
