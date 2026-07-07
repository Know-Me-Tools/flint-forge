# Plan — p11-api-stability

**Phase:** 11 — API Stability
**Authored:** 2026-07-06
**Change backend:** OpenSpec
**Changes:** 6 ordered
**Seeded from:** `assessment.md`

---

## Ordering Rationale

```
p11-c001 (#[non_exhaustive] + A2UI docs)   ─┐ both P0, fully independent
p11-c002 (WIT @since + Kiln docs)          ─┘ → run in parallel if subagents available
        ↓
p11-c003 (SDK 1.0.0 + MIGRATION.md)         depends on c001+c002 for accurate docs
        ↓
p11-c005 (Dockerfile entrypoints)            P1, no deps — can run after c003 or in parallel
p11-c004 (k6 baselines annotation)           P1, blocked on staging for full value
        ↓
p11-c006 (staging token rotation)            P2, independent of all others
```

**c001 and c002 are fully independent** and can be dispatched to parallel subagents.
**c003 depends on c001+c002** only for documentation accuracy (MIGRATION.md references
the API docs). The Rust work in c003 (version bumps) is independent.
**c005** has no dependencies and can run after c003 in the same session.
**c004** and **c006** are independent and can run last.

---

## Change List

### 1. `p11-c001-a2ui-api-freeze` — P0 — Execute first

**Scope:**
- Add `#[non_exhaustive]` to 9 public enums across `fdb-domain`, `fdb-app`,
  `fdb-reflection`, `fke-domain`, `forge-policy`
- Fix any exhaustive `match` arms that break (expect zero — all are library-internal)
- `mkdir -p docs/api/` + write `docs/api/a2ui.md` (~150 lines)
- Add `FLINT_A2UI_API_VERSION=1` to `.env.example`

**Expected effort:** Small. 9 one-line attribute additions; documentation authoring.

**Gate:** `cargo clippy --workspace -- -D warnings` clean; `cargo test --workspace` passes.

---

### 2. `p11-c002-kiln-abi-freeze` — P0 — Execute in parallel with c001

**Scope:**
- Add `@since(version = 0.1.0)` to 5 WIT interfaces in `wit/flint/host/world.wit`
- Add stability comment to `world edge-function` declaration
- Verify `cargo component build -p hello-component` still passes
- Write `docs/api/kiln-abi.md` (~200 lines)
- Add `FLINT_KILN_ABI_VERSION=1` to `.env.example`

**Expected effort:** Small. WIT edits are one-liners; documentation authoring is the bulk.

**Gate:** `cargo component build -p hello-component` passes; `cargo check --workspace` clean.

---

### 3. `p11-c003-sdk-v1-alignment` — P0 — Execute after c001+c002

**Scope:**
- `packages/flint-react/package.json`: `"version": "0.1.0"` → `"1.0.0"`
- `packages/flint_genui/pubspec.yaml`: `version: 0.1.0` → `1.0.0`
- Write `packages/flint-react/CHANGELOG.md`
- Write `packages/flint_genui/CHANGELOG.md`
- Write `MIGRATION.md` at workspace root

**Note on build gate:** `npm run build` / `flutter analyze` are not in CI; gate is
well-formed JSON/YAML validation + `cargo test --workspace` passes.

**Expected effort:** Small. Version bumps are single-line; changelogs and
MIGRATION.md are documentation authoring (~100 lines total).

---

### 4. `p11-c005-entrypoint-secrets` — P1 — Execute after c003

**Scope:**
- `docker/fdb-gateway/entrypoint.sh` — reads two secrets, sets two env vars, exec
- `docker/fke-server/entrypoint.sh` — reads one secret, sets env var, exec
- Update both Dockerfiles: `COPY` + `RUN chmod +x` + `ENTRYPOINT ["/entrypoint.sh"]`
- Remove `FLINT_JWT_SECRET_FILE` env annotation from prod compose
- Validate `docker compose config --quiet`

**Expected effort:** Small. Two ~20-line shell scripts; four Dockerfile line edits.

**Gate:** `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet` exits 0.

---

### 5. `p11-c004-k6-baselines` — P1 — Execute after c005

**Scope:**
- Add `BASELINE_DATE`/`BASELINE_SOURCE` constants + annotation block to `regression.js`
- Create `perf/results/.gitkeep`; add `perf/results/*.json` to `.gitignore`
- If staging is live: measure and record P50/P95/P99; update thresholds
- If staging not live: add TODO + annotate placeholder dates

**Expected effort:** Minimal (annotation only). Full value requires staging.

---

### 6. `p11-c006-staging-token-rotation` — P2 — Execute last

**Scope:**
- `scripts/mint_smoke_token.sh` — self-signed HS256, reads from three secret paths
- Update `deploy.yml` — mint step + `STAGING_JWT_SECRET` secret
- Remove `STAGING_SMOKE_TOKEN` from runbook secrets table; add `STAGING_JWT_SECRET`
- `docs/runbook.md §11` + `scripts/README.md`

**Expected effort:** Small. ~40-line shell script + workflow update + documentation.

---

## Build / Quality Gates (apply after every change)

```bash
cargo check --workspace            # fast loop gate
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo audit                        # must stay clean (0 CVSS ≥ 7.0)
docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet
```

`cargo component build -p hello-component` — after c002 only.

---

## MVP Gate Checklist

- [ ] `#[non_exhaustive]` on all 9 target enums; `docs/api/a2ui.md` written
- [ ] WIT `@since` annotations on 5 interfaces; `docs/api/kiln-abi.md` written
- [ ] `@flint/react` and `flint_genui` at version `1.0.0`; `MIGRATION.md` written
- [ ] k6 `regression.js` baseline annotation present (measurement deferred if no staging)
- [ ] Dockerfile entrypoints wire secrets; compose config validates
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
