# @flint/react Changelog

All notable changes to `@flint/react` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.0] — 2026-07-06

First stable release. Requires **Flint Forge ≥ v0.10.0** on the backend.

### Added

**Component registry (`src/registry/`)**
- `SLUG_MAP` — 55-slug component registry mapping every A2UI catalog slug to a
  React component. 27 slugs resolve to real components; 28 use tree-shakeable
  `Placeholder(slug)` stubs for components not yet implemented.
- All component slugs follow the `kebab-case` convention used by the A2UI catalog
  API (`GET /a2ui/v1/components`).

**Provider (`src/provider/`)**
- `useFlintRegistry()` — hook that connects to a live Quarry
  `GET /a2ui/v1/components` endpoint and returns a resolved component map. Accepts
  `baseUrl` and `token` options; uses `swr` for stale-while-revalidate caching.
- `FlintProvider` — React 19 context provider wrapping `useFlintRegistry`; enables
  downstream `useFlintComponent(slug)` calls anywhere in the tree.

**Tokens (`src/tokens/`)**
- `exportDesignSyncTokens(tokenMap)` — converts a Flint `DesignTokenMap` JSON
  blob into a flat CSS custom-property record, compatible with the W3C Design
  Tokens Community Group specification.

**AG-UI SSE (`src/ag-ui/`)**
- `useAgUiStream(url, options)` — hook for consuming the AG-UI SSE stream from
  `/agui/v1/events`. Automatically reconnects on network failure; exposes `events`,
  `isConnected`, and `error` state.

**Surface assembly (`src/surface/`)**
- `FlintSurface` — headless component that calls
  `POST /a2ui/v1/surfaces/assemble` and renders the assembled surface using
  the registry's resolved components.

**Components (`src/components/`)**
- Headless primitive set backed by Radix UI primitives (`@radix-ui/react-slot`).
  All components accept `asChild` for composition.

### Notes

- All API shapes follow the contract in `docs/api/a2ui.md`
- Requires React 19; peer dependency `react: ^19.0.0`
- Bundle size target: < 80 kB gzipped (enforced by `size-limit`)

---

## [0.1.0] — Initial scaffold

Internal scaffold release. Not published to npm.
