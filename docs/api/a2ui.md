# A2UI REST API Reference

**Base path:** `/a2ui/v1`  
**Current version:** `1` (`FLINT_A2UI_API_VERSION=1`)  
**Protocol spec:** A2UI v0.9.1 (`https://a2ui.org/schemas/catalog/v0.9.1`)

---

## Versioning Policy

The A2UI API follows a URL-based versioning scheme.

| Rule | Detail |
|---|---|
| Current prefix | `/a2ui/v1/` |
| Version env var | `FLINT_A2UI_API_VERSION=1` (informational; read by SDK clients to confirm compatibility) |
| Backward-compatible changes | New optional response fields, new optional query parameters, new enum variants in non-exhaustive types — **no version bump required** |
| Breaking changes | Removed fields, renamed paths, changed semantics, removed enum variants — **requires `/a2ui/v2/` and a deprecation notice** |
| Deprecation notice | At least one minor release before a `/v1/` endpoint is removed; the response will include `Deprecation: true` and `Sunset: <date>` headers |

SDK clients SHOULD read `FLINT_A2UI_API_VERSION` on startup and refuse to connect
if the value differs from the version they were compiled against.

---

## Authentication

Every route under `/a2ui/v1/` requires a valid JWT in the `Authorization` header.

```
Authorization: Bearer <JWT>
```

- JWTs are validated against the JWKS endpoint configured in `FLINT_GATE_JWKS_URL`.
- The `iss` claim must match `FLINT_GATE_ISSUER`.
- If `FLINT_GATE_AUDIENCE` is set, the `aud` claim is also validated.
- The JWT must contain a `flint.user_id` claim inside the payload JSON (populated by
  the identity service).
- Missing or invalid tokens return `401 Unauthorized`.
- Tokens that pass validation but lack the required role for an application resource
  return `403 Forbidden`.

---

## Rate Limits

| Surface | Default | Config variable |
|---|---|---|
| REST (this API) | 100 req/s per IP | `FLINT_RATE_LIMIT_REST` |
| Burst allowance | 10 additional req | `FLINT_RATE_LIMIT_BURST` |
| GraphQL | 20 req/s per IP | `FLINT_RATE_LIMIT_GRAPHQL` |

Requests that exceed the limit receive `429 Too Many Requests` with a
`Retry-After` header indicating the number of seconds to wait.

---

## Error Envelope

All error responses use the following JSON shape:

```json
{
  "error": "<machine-readable error code>",
  "message": "<human-readable description>"
}
```

The `message` field may be absent on simple errors (e.g. `{"error": "not found"}`).

| HTTP Status | `error` value | Meaning |
|---|---|---|
| 400 | `"missing field"` | Required field absent in request body |
| 400 | `"invalid config"` | Assembly rule or configuration is malformed |
| 401 | `"unauthorized"` | Missing or invalid JWT |
| 403 | `"forbidden"` | Valid JWT but insufficient role |
| 404 | `"not found"` | Resource does not exist or is not visible to the caller |
| 404 | `"component not found"` | Specific to component lookups |
| 404 | `"application not found"` | Specific to application lookups |
| 404 | `"catalog not found"` | Specific to catalog lookups |
| 404 | `"no binding"` | No A2UI binding matched the event context |
| 429 | `"rate limited"` | Request rate limit exceeded |
| 500 | `"internal server error"` | Unrecoverable server-side error |

---

## Component Schema — `ResolvedComponent`

`ResolvedComponent` is the primary type returned by component endpoints. It represents
a component definition with all per-application and per-design-system overrides applied.

| Field | Type | Nullable | Description |
|---|---|---|---|
| `slug` | `string` | no | Unique kebab-case identifier (e.g. `"data-grid"`, `"button"`) |
| `primitive_type` | `string` | no | SDK primitive name (e.g. `"DataGrid"`, `"Button"`) |
| `category` | `string` | no | Grouping category (e.g. `"data"`, `"action"`, `"layout"`) |
| `schema` | `object` | no | JSON Schema object describing accepted props |
| `description` | `string` | yes | Human-readable description of the component |
| `renderers` | `Renderers` | no | Which SDK renderers support this component (see below) |
| `prop_defaults` | `object` | no | Merged prop defaults from overrides; `{}` if none |
| `css_vars` | `object` | no | Merged CSS variable overrides; `{}` if none |
| `react_component` | `string` | yes | Overridden React import path; `null` = use SDK default |
| `flutter_widget` | `string` | yes | Overridden Flutter widget class name; `null` = use SDK default |
| `htmx_template` | `string` | yes | Overridden Askama template path; `null` = use SDK default |

### `Renderers`

| Field | Type | Default | Description |
|---|---|---|---|
| `react` | `boolean` | `true` | Component is available in the React SDK |
| `flutter` | `boolean` | `true` | Component is available in the Flutter SDK |
| `htmx` | `boolean` | `true` | Component is available in the HTMX renderer set |

A component with `flutter: false` is excluded from the Flutter SDK catalog. SDK
generators MUST honour these flags when producing platform-specific output.

### `DesignToken`

Design tokens follow the [W3C Design Tokens Community Group 2024](https://design-tokens.org/schema/2024)
format and are stored as nested JSONB in `flint_a2ui.design_systems.tokens`.

| Field | JSON key | Type | Description |
|---|---|---|---|
| `value` | `$value` | `string` | Token value (e.g. `"oklch(68% 0.21 250)"`) |
| `token_type` | `$type` | `string` | Token category (e.g. `"color"`, `"spacing"`) |

Example token map:

```json
{
  "color": {
    "primary": { "$value": "oklch(68% 0.21 250)", "$type": "color" },
    "surface": { "$value": "#ffffff", "$type": "color" }
  }
}
```

---

## Endpoints

### `GET /a2ui/v1/components`

List all components visible to the caller. Base components are always returned.
If `app_id` is supplied and the caller has a role assignment in that application,
app-specific components are included.

**Auth:** Bearer JWT required.

**Query parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `app_id` | `UUID` | no | Include app-specific components for this application |
| `category` | `string` | no | Filter to a specific category after SQL resolution |

**Response — 200 OK:**

```json
{
  "components": [
    {
      "id": "018e1f2b-...",
      "slug": "button",
      "category": "action",
      "primitive_type": "Button",
      "schema": { ... },
      "description": "A clickable button component"
    }
  ]
}
```

Note: the list response returns a summary shape (no `renderers`, `prop_defaults`,
`css_vars`, or renderer override fields). Use `GET /a2ui/v1/components/:slug` for
the full `ResolvedComponent` shape.

**Error codes:** 401, 429, 500.

---

### `GET /a2ui/v1/components/:slug`

Return a single component by its slug. The caller must be able to see it through
`flint_a2ui.resolve_components` (base components are always visible; app-scoped
components require a matching role assignment).

**Auth:** Bearer JWT required.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `slug` | `string` | Kebab-case component slug (e.g. `button`, `data-grid`) |

**Response — 200 OK:**

```json
{
  "component": {
    "id": "018e1f2b-...",
    "slug": "button",
    "category": "action",
    "primitive_type": "Button",
    "schema": { ... },
    "description": "A clickable button component",
    "renderers": { "react": true, "flutter": true, "htmx": true },
    "react_pkg": null,
    "flutter_pkg": null,
    "htmx_template": null
  }
}
```

**Error codes:** 401, 404 (`"component not found"`), 429, 500.

---

### `POST /a2ui/v1/components/search`

Hybrid text + semantic search over the component registry. When the `llm.embed()`
function is available in the database, `flint_a2ui.hybrid_search()` is used and
results are scored by cosine similarity. Otherwise the endpoint falls back to
PostgreSQL full-text search over `slug || description`.

**Auth:** Bearer JWT required.

**Request body (`application/json`):**

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | yes | — | Natural-language search query |
| `limit` | `integer` | no | `10` | Maximum number of results to return |
| `app_id` | `UUID` | no | `null` | Restrict results to components visible to this application |

```json
{
  "query": "button with icon",
  "limit": 5,
  "app_id": null
}
```

**Response — 200 OK:**

```json
{
  "results": [
    {
      "id": "018e1f2b-...",
      "slug": "icon-button",
      "category": "action",
      "primitive_type": "IconButton",
      "score": 0.91
    }
  ]
}
```

**Error codes:** 400, 401, 429, 500.

---

### `GET /a2ui/v1/components/bindings/:schema/:table`

Return all A2UI bindings (auto-generated and manual) for a database table. Used by
code-generation tools to discover which components are wired to which tables.

**Auth:** Bearer JWT required.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `schema` | `string` | PostgreSQL schema name (e.g. `public`) |
| `table` | `string` | Table name within the schema (e.g. `users`) |

**Response — 200 OK:**

```json
{
  "bindings": [
    {
      "id": "018e1f2c-...",
      "table_schema": "public",
      "table_name": "users",
      "binding_type": "list",
      "auto_generated": true,
      "config": { ... },
      "slug": "data-grid",
      "primitive_type": "DataGrid"
    }
  ]
}
```

**Error codes:** 401, 429, 500.

---

### `GET /a2ui/v1/applications`

List applications the caller has access to. System applications (`is_system = true`)
are always returned. Non-system applications require a `flint_a2ui.role_assignments`
entry for the caller's `user_id`.

Results are ordered: system applications first, then alphabetically by `slug`.

**Auth:** Bearer JWT required.

**Query parameters:** none.

**Response — 200 OK:**

```json
{
  "applications": [
    {
      "id": "018e1f2d-...",
      "slug": "flint-base",
      "name": "Flint Base",
      "description": "Built-in base application",
      "jwt_claims_template": { ... },
      "catalog_id": "flint-base",
      "is_system": true
    }
  ]
}
```

**Error codes:** 401, 429, 500.

---

### `GET /a2ui/v1/applications/:id`

Return a single application by UUID.

**Auth:** Bearer JWT required.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `id` | `UUID` | Application UUID |

**Response — 200 OK:**

```json
{
  "application": {
    "id": "018e1f2d-...",
    "slug": "my-app",
    "name": "My Application",
    "description": null,
    "jwt_claims_template": { "flint": { "role": "viewer" } },
    "catalog_id": "my-app",
    "is_system": false
  }
}
```

**Error codes:** 401, 404 (`"application not found"`), 429, 500.

---

### `GET /a2ui/v1/catalog/:catalog_id`

Serve the A2UI catalog as a JSON Schema object compatible with A2UI v0.9.1 and
CopilotKit's `<CopilotKit a2ui={{ catalog }}>` prop.

**Auth:** Bearer JWT required.

**Path parameters:**

| Parameter | Format | Description |
|---|---|---|
| `catalog_id` | `<slug>` or `<slug>/<version>` | Application slug and optional version tag. When version is omitted it defaults to `"1.0.0"`. Example: `flint-base/1.0.0` |

**Response — 200 OK:**

```json
{
  "$schema": "https://a2ui.org/schemas/catalog/v0.9.1",
  "catalogId": "https://forge.example.com/a2ui/v1/catalog/flint-base/1.0.0",
  "name": "Flint FLINT-BASE Catalog",
  "version": "1.0.0",
  "definitions": {
    "Button": {
      "type": "object",
      "properties": { ... },
      "description": "A clickable button component"
    },
    "DataGrid": { ... }
  }
}
```

The `definitions` object is keyed by `primitive_type`. Each value is the
component's JSON Schema definition with the optional `description` field injected.

The catalog includes base components plus all components scoped to the named
application. Callers that need a strict application-only catalog should filter
client-side or use `GET /a2ui/v1/applications/:id` to resolve the application UUID,
then `GET /a2ui/v1/components?app_id=<id>` for the full list.

**Error codes:** 401, 404 (`"catalog not found"` when the slug matches no known
application and no base components exist), 429, 500.

---

### `POST /a2ui/v1/surfaces/assemble`

Assemble an A2UI surface from an AG-UI event context. The assembler applies
application-specific assembly rules stored in `flint_a2ui.assembly_rules`, then
falls back to the default table → component binding in `flint_a2ui.bindings`.

This is the primary endpoint consumed by the AG-UI `Custom { name: "a2ui:surface" }`
event handler in frontend clients.

**Auth:** Bearer JWT required.

**Request body (`application/json`):**

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `event_type` | `string` | yes | — | AG-UI event type triggering the assembly (e.g. `"mount"`, `"navigate"`) |
| `event_context` | `object` | no | `{}` | Arbitrary JSON payload accompanying the event |
| `application_id` | `UUID` | no | `null` | Scope assembly rules to a specific application |

```json
{
  "event_type": "navigate",
  "event_context": {
    "path": "/users",
    "schema": "public",
    "table": "users"
  },
  "application_id": "018e1f2d-..."
}
```

**Response — 200 OK:**

The response shape is determined by the matched assembly rule. All surfaces share
a common envelope:

```json
{
  "surface_id": "018e1f30-...",
  "catalog_version": "1.0.0",
  "components": [
    {
      "instance_id": "018e1f31-...",
      "component_slug": "data-grid",
      "props": { "schema": "public", "table": "users" },
      "layout": { "order": 0, "region": "main" }
    }
  ],
  "metadata": {
    "rule_id": "018e1f32-...",
    "matched_by": "assembly_rule",
    "assembled_at": "2025-07-06T12:00:00Z"
  }
}
```

**Error codes:**

| Status | `error` | Condition |
|---|---|---|
| 400 | `"missing field"` | `event_type` absent or `event_context` missing a required field |
| 400 | `"invalid config"` | Assembly rule references a missing component or malformed config |
| 404 | `"no binding"` | No assembly rule matched and no default binding exists for the table |
| 401 | — | Missing or invalid JWT |
| 429 | — | Rate limit exceeded |
| 500 | `"internal server error"` | Database error during assembly |

---

## Design System Tokens (supplementary)

### `GET /a2ui/v1/design-systems/:id/tokens`

Return the design token map for a design system in W3C Design Token format.

**Auth:** Bearer JWT required.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `id` | `UUID` | Design system UUID |

**Response — 200 OK:** The raw JSONB token map stored in `flint_a2ui.design_systems.tokens`.

```json
{
  "color": {
    "primary": { "$value": "oklch(68% 0.21 250)", "$type": "color" }
  },
  "spacing": {
    "base": { "$value": "8px", "$type": "dimension" }
  }
}
```

**Error codes:** 401, 404, 429, 500.

---

## SDK Integration Notes

### CopilotKit

```tsx
import { CopilotKit } from "@copilotkit/react-core";

const catalog = await fetch("/a2ui/v1/catalog/my-app/1.0.0", {
  headers: { Authorization: `Bearer ${token}` },
}).then((r) => r.json());

<CopilotKit a2ui={{ catalog }}>
  {/* your app */}
</CopilotKit>
```

### Assembly event flow

1. Agent emits `Custom { name: "a2ui:surface", value: { event_type, event_context } }`.
2. Frontend client intercepts the event and calls `POST /a2ui/v1/surfaces/assemble`.
3. The response `components` array is rendered by the A2UI renderer for the active SDK.

### Version negotiation

```ts
const API_VERSION = parseInt(process.env.FLINT_A2UI_API_VERSION ?? "1", 10);
if (API_VERSION !== EXPECTED_VERSION) {
  throw new Error(`A2UI API version mismatch: expected ${EXPECTED_VERSION}, got ${API_VERSION}`);
}
```
