# Assessment — p8-sdk-completeness

**Phase:** p8-sdk-completeness
**Assessed:** 2026-07-04
**Previous phase:** p7b-kiln-production (6/6 done; clippy clean; 437 tests passing)

---

## Codebase Inventory

### What exists

| Artifact | Location | State |
|---|---|---|
| `@flint/react` package | `packages/flint-react/` | Source exists, `index.ts` exports components; build scripts present |
| `flint_genui` Dart package | `packages/flint_genui/` | Exists; `SseClient` has no reconnect logic |
| HTMX renderer | `crates/fdb-gateway/src/routes/htmx.rs` | 7 of 55 slugs have dedicated renderers |
| `design_import.rs` | `crates/fdb-gateway/src/routes/design_import.rs` | `design_md` + `w3c_tokens` formats; no ZIP |
| `exportDesignSyncTokens` | `packages/flint-react/src/tokens/` | `FlintTokens.ts` + `useDesignTokens.ts` exist; no export function |
| Skill catalog | `skills/flint-ui/catalogs/components.md` | 55 slugs documented |
| CI script | `scripts/ci-check.sh` | Runs fmt + clippy; **no GitHub Actions workflow** |
| Dockerfiles | `docker/postgres/`, `images/postgres18/` | Postgres only; **no gateway or kiln Dockerfiles** |

---

## Gap Analysis by Goal

### G1 — `@flint/react` SDK Completeness (P0)

**Current state:** `packages/flint-react/src/index.ts` exports components under semantic group names (`Stack`, `Card`, `DataGrid`, `Button`, etc.) but the naming diverges from the 55 catalog slugs. The catalog uses `container`, `row`, `column`, `data-grid`, `text-input`, `number-input`; the SDK exports `Stack`, `Grid`, `TextField`, `Form` etc. There is no `exportDesignSyncTokens()` function.

**Gap:**
- Catalog slugs (`container`, `row`, `column`, `grid`, `stack`, `divider`, `spacer`, `scroll-area`, …) are not directly exported as named exports — agents calling `import { DataGrid } from '@flint/react'` work, but `import { container } from '@flint/react'` does not
- `useFlintRegistry()` hook needs to be verified/added (only `useFlint()` is exported)
- Bundle size audit not yet run (`size-limit` config present in `package.json`)
- `exportDesignSyncTokens()` is missing (separate from G5)

**Work required:**
- Add re-exports mapping catalog slugs to PascalCase exports (`export { DataGrid as 'data-grid' }` — or add a `fromSlug(slug)` utility)
- Verify `useFlint()` satisfies the `useFlintRegistry()` use case or add a separate hook
- Run `npm run size` and confirm < 80 KB gzipped
- Add `exportDesignSyncTokens()` stub that calls `GET /a2ui/v1/design-systems/:id/tokens` (overlap with G5)

**Effort:** Medium — mostly TypeScript; bundle audit is `npm run size` once deps are correct.

---

### G2 — Flutter SDK SSE Reconnect (P0)

**Current state:** `packages/flint_genui/lib/src/transport/sse_client.dart` has a single `listen()` call inside `connect()`. On connection error, `controller.addError(e)` is called and the stream ends. There is **no reconnect logic**. The `catch` block closes the client and that's it.

**Gap:**
- No exponential backoff retry loop in `SseClient`
- `FlintA2uiTransport` (`flint_transport.dart`) does not invoke reconnect
- No `FlintCatalog.refresh()` method
- `FlintThemeData` token override from `component_overrides` not wired

**Work required:**
- Wrap `listen()` in a retry loop with backoff: `Duration backoff = 3s; while(true) { try { await listen(); } catch(e) { await Future.delayed(backoff); backoff = min(backoff * 2, 60s); } }`
- Expose `reconnectAttempts` counter in `SseClient` state
- Add `FlintCatalog.refresh()` that forces a catalog reload by re-fetching the catalog URL
- Wire `FlintThemeData` overrides: when `application_id` is set, fetch `component_overrides` from the REST API and merge into rendered widget props

**Effort:** Medium — pure Dart, no new deps needed.

---

### G3 — CI Pipeline (P0)

**Current state:** `scripts/ci-check.sh` runs `cargo fmt --check` + `cargo clippy --workspace`. No GitHub Actions workflows exist (`.github/` directory absent). No Dockerfiles for `fdb-gateway` or `fke-server`.

**Gap:**
- `.github/workflows/ci.yml` does not exist
- `cargo component build -p hello-component` not in CI gate
- No Docker build for `fdb-gateway` or `fke-server`
- No Docker publish to `ghcr.io`

**Work required:**
- Create `.github/workflows/ci.yml` — on `push` + `pull_request`:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo component build -p hello-component`
- Create `docker/fdb-gateway/Dockerfile` and `docker/fke-server/Dockerfile`
- Add `.github/workflows/docker.yml` — on `push` to `main`: build + push to `ghcr.io/prometheus-ags/flint-forge`

**Effort:** Medium — standard GitHub Actions boilerplate + 2 Dockerfiles.

---

### G4 — HTMX Remaining 48 Component Renderers (P1)

**Current state:** `crates/fdb-gateway/src/routes/htmx.rs` has dedicated renderers for 7 slugs: `data-grid`, `form`, `button`, `text`, `card`, `tabs`, plus a generic fallback. The remaining 48 slugs fall through to the generic JSON-inspect card.

**Gap:** All 48 remaining slugs need HTML renderers. The pattern is established (see `render_button`, `render_form`, etc.) — it's mechanical repetition across 6 categories:
- **Input (14):** `text-input`, `number-input`, `select`, `multi-select`, `date-picker`, `checkbox`, `radio`, `toggle`, `textarea`, `file-upload`, `search-input`, `color-picker`, `slider`, (form already done)
- **Action (5):** `action-bar`, `dropdown-menu`, `context-menu`, `fab`, `link`
- **Navigation (5):** `nav-bar`, `sidebar`, `breadcrumb`, `pagination`, `stepper`
- **Feedback (8):** `alert`, `toast`, `modal`, `dialog`, `loading-spinner`, `progress-bar`, `empty-state`, `error-boundary`
- **Data-display (10):** `data-table`, `badge`, `tag`, `avatar`, `stat-card`, `timeline`, `code-block`, `json-viewer`, `list`, `detail-view`
- **Layout (6):** `container`, `row`, `column`, `grid`, `stack`, `divider`, `spacer`, `scroll-area` (8, minus 2 already done elsewhere)

**Work required:** Implement `render_<slug>()` functions following the same pattern as the 7 existing renderers. Since `htmx.rs` is already ~500 lines, split into `htmx/` module directory.

**Effort:** Medium-high — mechanical but large (48 functions × ~10-15 lines each ≈ ~600 lines new code). Parallelisable across multiple subagents by category.

---

### G5 — Design Token Export (`exportDesignSyncTokens`) (P1)

**Current state:** No `exportDesignSyncTokens()` function exists. The tokens infrastructure (`FlintTokens.ts`, `useDesignTokens.ts`) is present but doesn't reach out to the REST API for design system tokens.

**Gap:**
- No `GET /a2ui/v1/design-systems/:id/tokens` REST endpoint
- No `exportDesignSyncTokens(options)` TypeScript function
- The `flint_a2ui.design_systems.tokens` column exists (migration 0007) and stores W3C token JSON

**Work required:**
- Add `GET /a2ui/v1/design-systems/:id/tokens` handler in `fdb-gateway/src/routes/a2ui.rs` that reads `design_systems.tokens` and returns W3C format
- Add `export async function exportDesignSyncTokens({ catalogUrl, systemId? })` in `packages/flint-react/src/tokens/exportDesignSyncTokens.ts`
- Export from `index.ts`

**Effort:** Small-medium — 1 REST endpoint (simple SELECT) + 1 TypeScript function.

---

### G6 — OpenDesign ZIP Import (P1)

**Current state:** `design_import.rs` handles `format: "design_md"` and `format: "w3c_tokens"`. A `format: "claude_design_zip"` path does not exist. The `zip` crate is not a workspace dep.

**Gap:**
- No ZIP extraction code
- `zip` crate not in workspace
- No `format: "claude_design_zip"` arm in `match body.format.as_str()`

**Work required:**
- Add `zip = "2"` to `[workspace.dependencies]` (check cargo search first)
- Add `import_claude_design_zip(state, body)` fn: extract bytes from base64 body → `zip::ZipArchive::new()` → find `DESIGN.md` entry → pass text to `parse_design_md()` → same flow as `import_design_md()`
- Add `"claude_design_zip" => import_claude_design_zip(state, body).await` arm
- Unit test: base64-encoded minimal ZIP containing a `DESIGN.md` → verifies parsed tokens returned

**Effort:** Small — ~60 lines; the ZIP extraction is the new part, everything else is a call to existing functions.

---

### G7 — Claude Skill Gate Tests (P2)

**Current state:** `skills/flint-ui/catalogs/components.md` has 55 slugs documented. No automated test verifies these against the live DB. The `claude plugin install` command isn't yet documented with a verified example.

**Gap:**
- No Rust integration test that compares `catalogs/components.md` slugs against `SELECT slug FROM flint_a2ui.components WHERE is_base = true`
- Installation path not verified end-to-end

**Work required:**
- Add a `#[cfg(feature = "integration")]` test in `fdb-gateway` or a standalone script that reads the catalog markdown and compares against the DB
- Document `claude plugin install` path in `skills/flint-ui/SKILL.md`

**Effort:** Low — mostly documentation + a small integration test.

---

## Dependency Map

```
G3 (CI pipeline)         — independent
G1 (@flint/react)        — independent (TypeScript-only)
G2 (Flutter reconnect)   — independent (Dart-only)
G5 (token export)        — independent (REST endpoint + TS function)
G6 (ZIP import)          — independent (Rust-only, design_import.rs)
G4 (HTMX 48 renderers)   — independent; large, parallelise by category
G7 (skill gate tests)    — depends on G1 export accuracy + live DB
```

All changes are independent. G4 is the largest and benefits most from parallel subagents.

---

## Risk Register

| Risk | Likelihood | Severity | Mitigation |
|---|---|---|---|
| `@flint/react` bundle exceeds 80 KB after adding 55 exports | MEDIUM | MEDIUM | Tree-shake all component code; use lazy-loaded registrations |
| HTMX `htmx.rs` exceeds 500 lines when adding 48 renderers | HIGH | LOW | Split into `htmx/` directory module before adding (mandatory) |
| `zip` crate v2 API incompatible with base64-input approach | LOW | LOW | Use `std::io::Cursor` as the `ZipArchive` reader |
| Flutter SSE reconnect loop fails on connection refused (no backoff cap) | LOW | MEDIUM | Hard-cap at 60 s; add `maxAttempts: 10` option |
| Docker image for `fke-server` pulls wasmtime at build time (large) | MEDIUM | LOW | Use multi-stage Dockerfile with cargo-chef layer caching |

---

## Assessment Summary

| Goal | Gap Size | Effort | Ready? |
|---|---|---|---|
| G1 `@flint/react` completeness | Medium — slug export mapping + bundle audit | Medium | ✅ |
| G2 Flutter reconnect | Medium — reconnect loop + catalog refresh | Medium | ✅ |
| G3 CI pipeline | Medium — GitHub Actions + 2 Dockerfiles | Medium | ✅ |
| G4 HTMX 48 renderers | Large — 48 functions, mechanical | High | ✅ (parallelise by category) |
| G5 Design token export | Small-Med — 1 REST endpoint + 1 TS fn | Small-Med | ✅ |
| G6 ZIP import | Small — 60 lines Rust + zip crate | Small | ✅ |
| G7 Skill gate tests | Low — integration test + docs | Low | ✅ (after G1) |

**No external blockers.** All 7 changes are implementable against the current codebase.

**Handoff to plan:** G3 (CI) + G5 (token export) + G6 (ZIP import) are the smallest changes and should be done first. G1 and G2 are medium effort and can run in parallel. G4 (HTMX) is the largest and should be parallelised across 3 subagents by category. G7 last.
