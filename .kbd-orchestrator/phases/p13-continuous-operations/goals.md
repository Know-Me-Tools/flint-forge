# Goals — p13-continuous-operations

## Phase Summary

Post-release steady-state operations for Flint Forge `v1.0.0`. This phase has
no fixed end date — it runs until a meaningful feature set or breaking change
justifies a new development cycle (`p14-*`). Changes are triggered by external
events: CVE advisories, dependency updates, monitoring insights, or operator
demand for new functionality.

The CI gates (`cargo audit`, `cargo clippy --workspace -- -D warnings`,
`cargo test --workspace`) are the continuous health signal. This phase exists
to close the two open debt items from p12 and establish the operational
rhythms for a released product.

Seeded from: `p12-v1-release/reflection.md` → "Recommended Next Phase"

---

## Changes (5 standing, triggered by events)

### P0 — Close inherited debt

- **G1 — p13-c001-k6-baselines-measure:**
  Run k6 scripts against a live staging stack; record measured P50/P95/P99;
  update `regression.js` thresholds and `docs/performance.md`.

  **Trigger:** Staging host becomes available.
  **Gate:** `regression.js` exits 0 against staging.
  **Reference:** `openspec/changes/p12-c001-k6-measure/proposal.md`

### P1 — Standing operational rhythms

- **G2 — p13-c002-dependency-maintenance:**
  Monthly pass: `cargo update && cargo audit`.
  - If `cargo audit` finds new unfixed CVSS ≥ 7.0 advisories not in the
    allowlist: upgrade the affected crate or add a justified allowlist entry
    with a 90-day expiry.
  - Update `.cargo/audit.toml` expiry dates approaching their deadline.
  - Commit: `chore(deps): monthly dependency update <YYYY-MM>`.

  **Trigger:** Monthly cadence or any new CVSS ≥ 7.0 advisory surfaced by CI.

- **G3 — p13-c003-monitoring-review:**
  After 30 days of production traffic, review Prometheus alert thresholds:
  - Compare `HighErrorRate` (> 1%), `HighP99Latency` (> 500 ms),
    `HighDbConnections` (> 8) against real observed values.
  - Tune thresholds in `observability/alerts.rules.yml` to reduce noise while
    preserving signal.
  - Update `docs/performance.md` with observed steady-state values.

  **Trigger:** 30 days post-deploy or first pager/alert fatigue report.

- **G4 — p13-c004-api-versioning-gate:**
  Establish and document the process for incrementing API versions:
  - Write `docs/api/versioning.md` — policy for when `FLINT_A2UI_API_VERSION`
    and `FLINT_KILN_ABI_VERSION` must be incremented (breaking change checklist).
  - Add a CI check that fails if `docs/api/a2ui.md` or `docs/api/kiln-abi.md`
    is modified without bumping the corresponding version variable in
    `.env.example`.
  - Gate: `cargo clippy + cargo test` passes with the new CI check.

  **Trigger:** First proposed breaking API change, or proactively in the first
  sprint after v1.0.0.

### P2 — Next development cycle seed

- **G5 — p13-c005-v1.1.0-planning:**
  Collect roadmap items from production usage, bug reports, and operator
  feedback. Produce a `docs/ROADMAP.md` with prioritised items for the
  `v1.1.0` cycle.
  - Group items by subsystem (Quarry, Kiln, SDK, ops).
  - Tag each item: `breaking` / `additive` / `fix` / `ops`.
  - Estimate scope (small / medium / large).
  - Trigger: when 3+ items are identified, open a new `p14-*` phase.

  **Trigger:** Organic — when there is enough demand to justify a new phase.

---

## Phase Complete When

This phase does not have a fixed completion gate. It transitions to `p14-*`
when:

- G5 produces a `docs/ROADMAP.md` with ≥ 3 prioritised items, OR
- A significant breaking change or new feature cluster is approved, OR
- The operator explicitly calls `/kbd-new-phase p14-*`

---

## Standing CI Gates (always active)

These run on every push to `main` and every PR, regardless of which goal
is being worked:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

---

## Open Debt Inherited from p12

| Item | Source | Status |
|---|---|---|
| k6 baselines TBD | p12-c001 (deferred) | G1 resolves |
| Grafana DB connections panel (sqlx) | p10-c004 | Deferred — no resolution path yet |
