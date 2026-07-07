# Goals — p8-sdk-completeness

## Phase Summary

Complete the developer-facing SDK surface across `@flint/react`, `flint_genui`, and the
HTMX renderer. The Kiln runtime and Quarry gateway are now fully hardened (p6b + p7b).
The highest leverage remaining work is SDK quality: prop types, reconnect logic, missing
component renderers, design token export, and a working CI pipeline.

Seeded from: `p7b-kiln-production/reflection.md` → "Recommended Next Phase"

---

## Changes (7 planned)

### P0 — Must ship

- **G1 — p8-c001-react-sdk-completeness:**
  `@flint/react` hardening — complete all missing named exports (all 55 component slugs
  as PascalCase exports), verify prop type accuracy against live DB schema, add
  `useFlintRegistry()` hook, run bundle size audit and confirm < 80 KB gzipped target.
  Document all public API in JSDoc. Lives in `crates/forge-cli` React SDK source or the
  TypeScript packages directory.

- **G2 — p8-c002-flutter-sdk-reconnect:**
  `flint_genui` Dart hardening — implement `FlintA2uiTransport` SSE reconnect with
  exponential backoff (3 s → 6 s → 12 s → 24 s, cap 60 s). Apply `FlintThemeData`
  token overrides from the `component_overrides` table (introduced in p5-c013 migration
  0007). Add `FlintCatalog.refresh()` to force a catalog reload without restarting the
  transport.

- **G3 — p8-c003-ci-pipeline:**
  CI pipeline — `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`
  on every PR via GitHub Actions. Add `cargo component build -p hello-component` gate
  (verifies the Kiln WASM example still compiles). Docker image build steps for
  `fdb-gateway` and `fke-server`. Store images in GitHub Container Registry
  (`ghcr.io/prometheus-ags/flint-forge`).

### P1 — Should ship

- **G4 — p8-c004-htmx-remaining-components:**
  HTMX renderer — implement dedicated renderers for the remaining 48 component slugs
  (beyond the 7 in `routes/htmx.rs`). Batch them by category: input (14 components),
  action (5), navigation (5), feedback (7), data-display (10), layout (7). Each renderer
  follows the same pattern as the existing 7.

- **G5 — p8-c005-design-token-export:**
  `@flint/react` `exportDesignSyncTokens()` — returns the active catalog's design tokens
  in W3C Design Token Community Group 2024 format, ready for `/design-sync`. Reads from
  `flint_a2ui.design_systems` via `GET /a2ui/v1/design-systems/:id/tokens`. Used by
  Claude Design and OpenDesign integrations.

- **G6 — p8-c006-opendesign-zip-import:**
  Complete the Claude Design ZIP import path in `POST /a2ui/v1/design-systems/import`.
  Currently only `format: "design_md"` and `format: "w3c_tokens"` are handled. Add
  `format: "claude_design_zip"` — extract `DESIGN.md` from the ZIP archive, pass
  through the existing `parse_design_md()` parser. Uses `zip` crate (add to workspace).

### P2 — Ship if capacity allows

- **G7 — p8-c007-claude-skill-gate-tests:**
  Live gate tests for the `skills/flint-ui/` Claude Code skill:
  - `cargo test -p hello-component` must pass (component slug accuracy)
  - Verify `catalogs/components.md` slugs match `SELECT slug FROM flint_a2ui.components WHERE is_base = true`
  - Document installation: `claude plugin install flint-ui@prometheus-ags/flint-forge`

---

## Phase Complete When (MVP gate)

- [ ] `@flint/react` exports all 55 component slugs and passes bundle size audit
- [ ] `FlintA2uiTransport` reconnects with exponential backoff (tests pass)
- [ ] CI pipeline runs `cargo test --workspace` green on PR
- [ ] Docker images build for `fdb-gateway` and `fke-server`
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Dependencies

### All resolved
- `flint_a2ui.component_overrides` table — ✅ migration 0007 (p5-c013)
- `flint_a2ui.design_systems` table — ✅ migration 0007
- `parse_design_md()` parser — ✅ p5-c013 `fdb-app/src/a2ui/design_md_parser.rs`
- `POST /a2ui/v1/design-systems/import` endpoint — ✅ p5-c013 `routes/design_import.rs`
- `skills/flint-ui/` skill package — ✅ p5-c015

### New dependencies
- OQ-P8-1: `zip = "2"` crate for ZIP archive extraction in G6 — check workspace before adding
- OQ-P8-2: GitHub Actions `cargo-component` installation for G3 CI gate
- OQ-P8-3: `ghcr.io` credentials for Docker publish in G3 CI
