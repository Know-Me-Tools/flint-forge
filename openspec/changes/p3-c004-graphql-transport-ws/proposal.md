# p3-c004 — GraphQL WebSocket: graphql-transport-ws Upgrade

## Change ID
`p3-c004-graphql-transport-ws`

## Phase
`p3-graphql-hybrid-engine`

## Priority
P0 — Requires p3-c007 (GraphQlCompiler + CompiledState.subscription_schema)

## Problem Statement

`GET /graphql` does not exist. Subscription clients using the `graphql-transport-ws`
protocol (GraphQL over WebSocket) have nowhere to connect. `async-graphql-axum`'s
`GraphQLSubscription` handler is not wired.

## Scope

### In Scope
- Register `GET /graphql` in `fdb-gateway/src/main.rs` as a WebSocket upgrade handler
- Use `async-graphql-axum::GraphQLSubscription` to handle the WebSocket upgrade
- Extract bearer token from WebSocket upgrade `Authorization` header (or `connection_init` payload)
- Build `RlsContext` from bearer via `fdb-auth::rls_from_bearer()`
- Pass `subscription_schema` from `CompiledState` to the `GraphQLSubscription` handler
- The handler reads `CompiledState.subscription_schema` from `StateManager` per connection

### Out of Scope
- Actual subscription resolver bodies (those connect to `FabricChangeSource` in p3-c002)
- Introspection merge (p3-c003)

## Design

### GET /graphql route (fdb-gateway/src/main.rs)

```rust
use async_graphql_axum::GraphQLSubscription;

async fn graphql_ws_handler(
    State(state): State<GatewayState>,
    headers: axum::http::HeaderMap,
    ws: axum::extract::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    let compiled = state.state_manager.current();
    let schema = match &compiled.subscription_schema {
        Some(s) => s.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE, "no schema").into_response(),
    };
    // Bearer from Sec-WebSocket-Protocol header or later from connection_init
    // For now: extract from Authorization header on upgrade request
    let bearer = extract_bearer(&headers);
    // Build RlsContext eagerly on connection (subscribe-time gate)
    // If bearer is None, use anonymous RlsContext
    ws.on_upgrade(move |socket| async move {
        GraphQLSubscription::new(schema)
            .serve(socket)
            .await
    })
}
```

The subscription resolvers (in `FabricChangeSource`) will receive the `RlsContext`
via async-graphql context injection (p3-c002). For now, the handler wires the
WebSocket protocol; actual subscription data flows in p3-c002.

### Route registration

```rust
.route("/graphql", get(graphql_ws_handler).post(handle_graphql_query))
```

Both operations share the `/graphql` path; method routing distinguishes them.

## Security Contracts
- WebSocket handler MUST NOT log bearer tokens from the upgrade handshake
- The bearer on the upgrade request (HTTP) has the same verification path as REST
- Each subscription connection gets its OWN `RlsContext` — context MUST NOT be shared between connections
- The `subscription_schema` is read-only from `CompiledState` — no mutation possible

## Acceptance Criteria
- `GET /graphql` route registered in `fdb-gateway`
- WebSocket upgrade succeeds (connection establishes without error)
- `graphql-transport-ws` protocol negotiation works (client can send `connection_init`)
- `cargo check --workspace` GREEN; clippy pedantic passes
- Unit test: `test_graphql_ws_route_registered` — verify route exists via router inspection
