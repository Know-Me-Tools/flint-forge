# flint_genui Changelog

All notable changes to `flint_genui` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.0] — 2026-07-06

First stable release. Requires **Flint Forge ≥ v0.10.0** on the backend.

### Added

**SSE transport (`lib/src/transport/`)**
- `FlintSseClient` — SSE client that connects to the Quarry AG-UI event stream
  (`/agui/v1/events`). Features:
  - Automatic reconnect with exponential backoff (configurable `initialBackoff`,
    default 1 s; max 30 s)
  - `refresh()` method for manual connection reset
  - Injectable `clientFactory` for test isolation (avoids real HTTP in unit tests)
  - Emits typed `AgUiEvent` objects matching the AG-UI protocol

**Catalog (`lib/src/catalog/`)**
- `FlintCatalog` — loads the A2UI component catalog from
  `GET /a2ui/v1/components`. Returns a `Map<String, FlintComponent>` keyed by
  slug. Caches the result in memory; call `refresh()` to invalidate.

**Surface (`lib/src/flint_surface.dart`)**
- `FlintSurface` — Flutter widget that calls
  `POST /a2ui/v1/surfaces/assemble` with an event context and renders the
  assembled surface using the component catalog.

**Tokens (`lib/src/tokens/`)**
- `FlintTokens` — parses the W3C Design Token JSON from
  `GET /a2ui/v1/design-systems/:id/tokens` and exposes typed color, spacing,
  and typography token accessors.

**Components (`lib/src/components/`)**
- Stateless component primitives for the Flutter ecosystem. All components accept
  `FlintTheme` for token-driven styling.

**Animations (`lib/src/animations/`)**
- `FlintAnimations` — curated animation presets (fade, slide, scale) aligned with
  the AG-UI motion tokens.

### Notes

- All API shapes follow the contract in `docs/api/a2ui.md`
- Requires Flutter ≥ 3.24.0 and Dart SDK ≥ 3.5.0
- No dependency on Gemini, Firebase, or Google AI services

---

## [0.1.0] — Initial scaffold

Internal scaffold release. Not published to pub.dev.
