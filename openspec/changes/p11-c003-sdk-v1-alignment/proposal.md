# p11-c003 — SDK v1.0 Alignment

**Phase:** 11 — API Stability  **Priority:** P0  **Depends on:** p11-c001, p11-c002

## Problem

`@flint/react` and `flint_genui` are both at version `0.1.0`. There are no SDK
changelogs and no `MIGRATION.md` at the workspace root. Downstream skill authors
and UI builders have no stable version signal and no upgrade path documentation.

## Changes

### Version bumps

- `packages/flint-react/package.json`: `"version": "0.1.0"` → `"version": "1.0.0"`
- `packages/flint_genui/pubspec.yaml`: `version: 0.1.0` → `version: 1.0.0`

### `packages/flint-react/CHANGELOG.md`

```markdown
# @flint/react Changelog

## [1.0.0] — 2026-07-06

### Added
- 55-slug component registry (`SLUG_MAP`) covering all A2UI catalog components
- `useFlintRegistry()` hook — connects to a live Quarry `/a2ui/v1/components`
  endpoint and returns the resolved component map
- Design token export utility (`exportDesignSyncTokens`)
- Placeholder components for unimplemented slugs (tree-shakeable)

### Notes
- Requires Flint Forge ≥ v0.10.0 on the backend
- All component props follow the A2UI `ResolvedComponent` schema
  (see `docs/api/a2ui.md`)
```

### `packages/flint_genui/CHANGELOG.md`

```markdown
# flint_genui Changelog

## [1.0.0] — 2026-07-06

### Added
- `FlintSseClient` with automatic reconnect and exponential backoff
- `refresh()` method for manual connection reset
- Catalog loader (`FlintCatalog`) for component resolution
- Injectable `clientFactory` and `initialBackoff` for test isolation

### Notes
- Requires Flint Forge ≥ v0.10.0 on the backend
- SSE stream emits AG-UI `AgUiEvent` payloads
```

### `MIGRATION.md` (workspace root)

Covers the v0.10.0 → v1.0.0 (p10→p11) delta for downstream consumers:

- Summary of breaking changes (none — p11 is additive)
- New `#[non_exhaustive]` on 9 enums — match arms must add `_` wildcard
- WIT `@since` annotations — informational; no compile-time impact
- `DATABASE_URL` construction now via Dockerfile entrypoint (p11-c005)
- SDK versions bumped from `0.1.0` to `1.0.0`
