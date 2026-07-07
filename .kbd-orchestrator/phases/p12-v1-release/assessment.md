# Assessment — p12-v1-release

**Phase:** 12 — v1.0.0 Release
**Assessed:** 2026-07-06
**Assessor:** OpenCode / KBD automated assess
**Changes in scope:** 2 (p12-c001 + p12-c002)
**Prior phase:** p11-api-stability (6/6 complete)

---

## Summary

The codebase is v1.0.0-ready today. The only gap blocking c002 is the version
bump (`0.10.0 → 1.0.0`) and the tag itself. G1 (k6 baselines) is blocked on a
live staging stack that is not available in this environment; the goals.md
fallback applies — waive G1, document TBD in the release notes. One additional
finding from assessment: `docker.yml` triggers on `branches: [main]` only, not
on version tags, so `ghcr.io/.../fdb-gateway:v1.0.0` would not be published
automatically. This is a small one-line fix that belongs in c002.

---

## Goal-by-Goal Gap Analysis

### G1 — k6 Measured Baselines (`p12-c001`) — ⛔ BLOCKED (fallback applies)

**What exists:**
- `perf/k6/regression.js` with `BASELINE_DATE = 'TBD'` and aspirational thresholds
- 3 individual k6 scripts ready to run
- `docs/performance.md` with TBD placeholder table

**Blocker:** No live staging stack is reachable. `http://localhost:8080/healthz`
returns connection refused; no cloud host is configured.

**Fallback resolution (per goals.md):**

> "If staging cannot be brought up promptly, tag v1.0.0 with aspirational
> thresholds and document that baselines are TBD in the release notes."

**Decision: Waive p12-c001.** The k6 gate is `workflow_dispatch`-only and is
not in the required CI path. Document the TBD status clearly in the v1.0.0
release notes. A future operator action (`k6 run` against staging) will close
this debt without requiring a new release phase.

**Gaps (waived):**

| Gap | Severity | Status |
|---|---|---|
| `BASELINE_DATE` / thresholds still TBD in `regression.js` | P0 | **Waived — staging unavailable** |
| `docs/performance.md` baseline table all TBD | P0 | **Waived — same blocker** |

---

### G2 — v1.0.0 Tag (`p12-c002`) — ⚠️ NOT STARTED + one additional finding

**What exists:**
- `[workspace.package] version = "0.10.0"` in `Cargo.toml` — needs bump
- `@flint/react` already at `1.0.0` ✅ (p11-c003)
- `flint_genui` already at `1.0.0` ✅ (p11-c003)
- `git cliff` installed; `cliff.toml` configured
- No `v1.0.0` tag exists
- 6 user-facing conventional commits since `v0.10.0` (clean CHANGELOG preview confirmed)

**`git cliff --unreleased` preview (already verified):**

```
## [Unreleased]
### Bug Fixes
- fix(security): crossbeam-epoch 0.9.18→0.9.20 (RUSTSEC-2026-0204)
### Features
- feat(perf,ops): k6 baseline annotation + staging token rotation
- feat(ops): Dockerfile entrypoint secrets wiring
- feat(api): A2UI + Kiln ABI freeze
### Maintenance (chore — filtered by cliff.toml)
- (chore(release): v1.0.0 — will be filtered out)
```

**Additional finding — `docker.yml` does not trigger on version tags (OQ-P12-2 resolved):**

```yaml
# current docker.yml — triggers on branch pushes only
on:
  push:
    branches: [main]
```

This means pushing `v1.0.0` tag will not automatically publish
`ghcr.io/.../fdb-gateway:v1.0.0` and `fke-server:v1.0.0`. Two options:
1. Add `tags: ['v[0-9]*']` to the `docker.yml` trigger → automatic per-tag images (**recommended**)
2. Accept commit-SHA-tagged images in the release notes

**Option 1 is 2 lines in `docker.yml`** and is the standard practice for versioned Docker releases. Include it in c002.

**Gaps:**

| Gap | Severity | Notes |
|---|---|---|
| `Cargo.toml` version `0.10.0` → `1.0.0` | P0 | Single-line bump |
| No `v1.0.0` tag | P0 | `git tag -a v1.0.0` |
| `docker.yml` doesn't trigger on tags | P0 | Add `tags: ['v[0-9]*']` to docker.yml |
| `CHANGELOG.md` not yet updated for v1.0.0 | P0 | `git cliff --tag v1.0.0 -o CHANGELOG.md` |
| No GitHub Release `v1.0.0` | P0 | `gh release create v1.0.0` |
| v1.0.0 release notes should note k6 TBD | LOW | Document in release body |

**Effort estimate:** Small. All mechanical steps — no code changes.

---

## Open Questions — Resolution

| OQ | Resolution |
|---|---|
| OQ-P12-1: Staging available? | **No.** Local stack not running; no cloud host configured. Waive p12-c001 per goals.md fallback. |
| OQ-P12-2: Docker image digests in release? | **Yes, via trigger fix.** Add `tags: ['v[0-9]*']` to `docker.yml`; images will be published when `v1.0.0` is pushed. Digests can be extracted from the completed CI run and added to the GitHub Release body. |

---

## Revised Plan (p12-c001 waived)

Since p12-c001 is blocked, p12 is reduced to a **single change: p12-c002-v1-tag**.

The plan-phase change list should be updated to reflect this: p12-c001 moves to
`status: "deferred"` with a note that it requires a live staging host.

**p12-c002 scope (expanded to include docker.yml fix):**

1. Update `docker.yml` — add `tags: ['v[0-9]*']` trigger (2-line change)
2. Bump `Cargo.toml` workspace version `0.10.0` → `1.0.0`
3. Generate `CHANGELOG.md` update: `git cliff --tag v1.0.0 -o CHANGELOG.md`
4. Commit: `chore(release): v1.0.0`
5. Tag: `git tag -a v1.0.0 -m "Flint Forge v1.0.0 — first stable API release"`
6. Push main + tag
7. Wait for `docker.yml` CI to complete, extract digests
8. Create GitHub Release with release notes noting k6 baselines TBD

---

## MVP Gate — Current Status

| Gate condition | Current state | Gap |
|---|---|---|
| k6 thresholds measured OR documented TBD | ⚠️ Documented TBD — waived | Waived |
| `Cargo.toml` version `1.0.0` | ❌ `0.10.0` | c002 |
| `git tag v1.0.0` pushed | ❌ absent | c002 |
| GitHub Release `v1.0.0` | ❌ absent | c002 |
| `cargo test --workspace` passes | ✅ 457 tests | — |
| `cargo clippy --workspace -- -D warnings` clean | ✅ | — |

**Two of six gate conditions already pass.** Four require c002. p12-c001 is waived.

---

*Assessment complete. Proceed to `/kbd-plan p12-v1-release`.*
