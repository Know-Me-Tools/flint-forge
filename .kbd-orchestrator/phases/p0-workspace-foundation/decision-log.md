# Decision Log — p0-workspace-foundation

---

### 2026-06-29 — pg_graphql PG18 deferral
**Decision:** Defer pg_graphql from Phase 0 (PG18 image) to Phase 2 (GraphQL implementation).  
**Rationale:** No released pg_graphql build for PG18 exists (supabase/pg_graphql#614). Build from source SHA is feasible but was deferred to avoid blocking Phase 0 gate.  
**Provenance:** Assessment (c002 gate).  
**Open question:** OQ-3 — strategy (build from source SHA / PG17 sidecar / wait) must be resolved before Phase 2 spec.  

---

### 2026-06-29 — WIT param type deferral (c003 open)
**Decision:** c003 (WIT contract freeze) remains open due to two spec divergences.  
**Divergences:**  
- `db.query` params: `list<string>` (actual) vs `list<json>` (spec §5.4 requires). Answer pending: is `list<json>` a WASM Component Model type? If not, `list<u8>` (serialized JSON bytes) is the correct representation.  
- `secrets.get`: returns `result<string, string>` (actual) vs Cedar-gated `resource secret { reveal: func() -> result<string, error> }` (spec requires). The resource type ensures the Kiln linker can attach Cedar capability gating at the WIT boundary.  
**Provenance:** Assessment (c003 gate).  
**Action:** Fix both divergences, install wasm-tools, build sample component, declare freeze.  

---

### 2026-06-29 — Supabase Storage gap acknowledged
**Decision:** Add Phase 6 (Storage) to the Forge phase plan. Not a blocking gap for Phases 1-5.  
**Rationale:** Supabase Storage is a production Supabase feature with no current Flint equivalent. The fke-store-s3 crate (Kiln component store) provides the S3 client foundation; a Postgres metadata table + Vault credentials + Keto ACL layer is the build-on-top approach.  
**Provenance:** Analyze (Supabase feature map, section 1).  

---

### 2026-06-29 — pg_cron addition to PG18 image
**Decision:** Add pg_cron to images/postgres18/Dockerfile as a Phase 1 addition.  
**Rationale:** Supabase ships pg_cron; Flint should too. One-line Dockerfile change. No new phase needed.  
**Provenance:** Analyze (Supabase feature map, section 3).  

---

### 2026-06-29 — Supabase-equivalent phase plan finalized
**Decision:** Adopt 8-phase plan (p0–p7, adding p6-storage) with cross-repo coordination points.  
**Key constraint:** FRF Phase 0 proto freeze (WatchEntityType RPC) must precede Forge Phase 1 start. Coordinate with flint-realtime-fabric team before p1 spec.  
**Provenance:** Analyze (section 3, cross-repo phase dependency map).  

---

### 2026-06-29 — flint-gate JWT contract: OQ-4 and OQ-5 are pre-Phase-1 blockers
**Decision:** OQ-4 (JWT claim shape) and OQ-5 (service-identity token format) must be resolved and documented in a shared contract file before Phase 1 spec is written.  
**Rationale:** `fdb-auth` (forge-identity::verify_and_build) verifies the flint-gate JWT. If the claim shape changes after p1 code is written, it requires coordinated changes in both repos. Pin it now.  
**Provenance:** Analyze (section 2, integration contracts OQ-4 and OQ-5).  
**Action:** Create `docs/contracts/jwt-contract.md` in flint-forge (referenced by flint-gate) before p1 spec.  
