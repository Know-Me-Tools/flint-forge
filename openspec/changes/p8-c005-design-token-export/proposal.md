# p8-c005 — Design Token Export (`exportDesignSyncTokens`)

**Phase:** 8 — SDK Completeness
**Priority:** P1
**Depends on:** none

## What this change delivers

- `GET /a2ui/v1/design-systems/:id/tokens` REST endpoint returning the design system's tokens in W3C format
- `exportDesignSyncTokens({ gatewayUrl, systemId, bearerToken })` TypeScript function in `@flint/react`
- Used by Claude Design `/design-sync` to pull design tokens from the Flint registry

## Design

### REST endpoint (Rust, `routes/a2ui.rs`)

```rust
/// GET /a2ui/v1/design-systems/:id/tokens
/// Returns the design system's tokens jsonb in W3C Design Token format.
pub async fn get_design_system_tokens(
    State(state): State<A2uiState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let tokens: Option<(sqlx::types::Json<Value>,)> = sqlx::query_as(
        "SELECT tokens FROM flint_a2ui.design_systems WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    match tokens {
        Some((t,)) => Ok(Json(t.0)),
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error":"design system not found"})))),
    }
}
```

Wire at `GET /a2ui/v1/design-systems/:id/tokens` behind `require_rls`.

### TypeScript function

```ts
// packages/flint-react/src/tokens/exportDesignSyncTokens.ts
export async function exportDesignSyncTokens(opts: {
  gatewayUrl: string;
  systemId: string;
  bearerToken: string;
}): Promise<Record<string, unknown>> {
  const resp = await fetch(
    `${opts.gatewayUrl}/a2ui/v1/design-systems/${opts.systemId}/tokens`,
    { headers: { Authorization: `Bearer ${opts.bearerToken}` } }
  );
  if (!resp.ok) throw new Error(`Failed to fetch tokens: ${resp.status}`);
  return resp.json();
}
```

Export from `packages/flint-react/src/index.ts`.
