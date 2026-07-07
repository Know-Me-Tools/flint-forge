# p11-c001 — A2UI API Freeze

**Phase:** 11 — API Stability  **Priority:** P0  **Depends on:** none

## Problem

5 public enums on the A2UI/reflection surface and 5 on the Kiln/policy surface
lack `#[non_exhaustive]`, making exhaustive match arms in downstream consumers
a semver hazard. `docs/api/` does not exist; there is no machine-readable or
human-readable contract for the A2UI HTTP API.

## Enums requiring `#[non_exhaustive]`

| Enum | Crate | File:line |
|---|---|---|
| `AgUiEvent` | `fdb-domain` | `src/lib.rs:108` |
| `ParseError` | `fdb-app` | `src/a2ui/design_md_parser.rs:67` |
| `ReflectionError` | `fdb-reflection` | `src/error.rs:4` |
| `EndpointKind` | `fdb-reflection` | `src/passes/endpoint_generation.rs:12` |
| `AssemblerError` | `fdb-reflection` | `src/compilers/a2ui.rs:18` |
| `Capability` | `fke-domain` | `src/lib.rs:12` |
| `CompilationStrategy` | `fke-domain` | `src/lib.rs:23` — already has `#[non_exhaustive]` ✓ |
| `TargetArch` | `fke-domain` | `src/lib.rs:30` |
| `Decision` | `forge-policy` | `src/lib.rs:16` |
| `PolicyLoadError` | `forge-policy` | `src/cedar.rs:47` |

`CompilationStrategy` already has `#[non_exhaustive]`; the remaining 9 need the
attribute added on the line immediately before the `pub enum` keyword.

## `docs/api/a2ui.md` content outline

The document covers the public A2UI HTTP API contract:

1. **Versioning policy** — `FLINT_A2UI_API_VERSION=1`; breaking changes require
   a new API version path prefix (`/a2ui/v2/…`)
2. **Component schema** — `ResolvedComponent` fields, slugs, renderer keys
3. `GET /a2ui/v1/components` — query params, auth, response shape
4. `GET /a2ui/v1/components/:slug` — path, response, 404 behaviour
5. `POST /a2ui/v1/components/search` — body shape, fuzzy search semantics
6. `GET /a2ui/v1/components/bindings/:schema/:table` — column binding shape
7. `GET /a2ui/v1/applications` — list shape
8. `GET /a2ui/v1/applications/:id` — application model shape
9. `GET /a2ui/v1/catalog/:catalog_id` — catalog shape
10. `POST /a2ui/v1/surfaces/assemble` — event context body, assembled surface response
11. **Authentication** — all routes require `Authorization: Bearer <JWT>`
12. **Errors** — error envelope shape, common status codes
