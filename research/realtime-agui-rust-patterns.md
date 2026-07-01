# Real-Time Schema Propagation, AG-UI/A2UI Patterns, and Atomic State Management in Rust

## Research Date: 2026-06-30

> **Research Scope:** This document covers three critical architectural patterns for building AI-native, real-time platforms: (1) PostgreSQL LISTEN/NOTIFY for reliable metadata change propagation, (2) AG-UI and A2UI standards for AI-generated dynamic interfaces, and (3) lock-free atomic state management in Rust async applications using ArcSwap, parking_lot, and tokio::sync::watch.

---

## 1. PostgreSQL LISTEN/NOTIFY for Real-Time Schema Propagation

### 1.1 Core Primitives

PostgreSQL provides three event-driven primitives that work together for real-time schema propagation:

- **Event Triggers** — Fire automatically when DDL commands execute (`CREATE`, `ALTER`, `DROP`, `COMMENT`, `GRANT`, `REVOKE`, `SELECT INTO`). They run inside the same transaction as the DDL, giving atomic guarantees: if the trigger fails, the schema change rolls back.
- **LISTEN/NOTIFY** — A built-in pub/sub system since PostgreSQL 6.4. Notifications deliver in sub-millisecond latency to all connected listeners, with payloads up to 8KB (as of PostgreSQL 9+). Notifications are delivered only after the transaction commits; if the transaction rolls back, the notification is never sent.
- **Outbox Pattern** — A durable table that persists events, enabling replay, observability, and at-least-once delivery even when listeners are disconnected.

### 1.2 DDL Event Capture Architecture

The recommended architecture combines an event trigger with an outbox table and LISTEN/NOTIFY for reliable propagation:

```sql
-- 1. Outbox table for durable event storage
CREATE TABLE platform.schema_changes (
    id           bigserial PRIMARY KEY,
    event_data   jsonb NOT NULL,
    created_at   timestamptz DEFAULT now(),
    processed    boolean DEFAULT false,
    processed_at timestamptz
);

-- Index for efficient polling fallback
CREATE INDEX idx_unprocessed ON platform.schema_changes (processed, created_at)
WHERE NOT processed;

-- 2. Event trigger function: captures DDL and writes to outbox + NOTIFY
CREATE OR REPLACE FUNCTION platform.log_and_notify()
RETURNS event_trigger AS $$
DECLARE
    rec     record;
    payload jsonb;
BEGIN
    FOR rec IN SELECT * FROM pg_event_trigger_ddl_commands() LOOP
        payload := jsonb_build_object(
            'event_id',        gen_random_uuid(),
            'command',         rec.command_tag,
            'object_identity', rec.object_identity,
            'schema_name',     rec.schema_name,
            'object_type',     rec.object_type,
            'timestamp',         now()
        );

        -- Durable outbox write
        INSERT INTO platform.schema_changes (event_data) VALUES (payload);

        -- Ephemeral notification (listeners get event_id, fetch full payload from outbox)
        PERFORM pg_notify('schema_changed', payload->>'event_id');
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- 3. Event trigger binding
CREATE EVENT TRIGGER capture_schema_changes
    ON ddl_command_end
    WHEN TAG IN (
        'CREATE TABLE', 'ALTER TABLE', 'DROP TABLE',
        'CREATE INDEX', 'DROP INDEX',
        'CREATE FUNCTION', 'ALTER FUNCTION', 'DROP FUNCTION',
        'CREATE SCHEMA', 'ALTER SCHEMA', 'DROP SCHEMA',
        'CREATE TYPE', 'ALTER TYPE', 'DROP TYPE',
        'CREATE VIEW', 'ALTER VIEW', 'DROP VIEW'
    )
    EXECUTE FUNCTION platform.log_and_notify();

-- 4. Separate trigger for DROP events (pg_event_trigger_ddl_commands() returns nothing on DROP)
CREATE OR REPLACE FUNCTION platform.notify_drops()
RETURNS event_trigger AS $$
DECLARE
    rec     record;
    payload jsonb;
BEGIN
    FOR rec IN SELECT * FROM pg_event_trigger_dropped_objects() LOOP
        payload := jsonb_build_object(
            'event_id',        gen_random_uuid(),
            'command',         'DROP',
            'object_identity', rec.object_identity,
            'schema_name',     rec.schema_name,
            'object_type',     rec.object_type,
            'is_cascade',      rec.is_cascade,
            'timestamp',         now()
        );
        INSERT INTO platform.schema_changes (event_data) VALUES (payload);
        PERFORM pg_notify('schema_dropped', payload->>'event_id');
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE EVENT TRIGGER capture_schema_drops
    ON sql_drop
    EXECUTE FUNCTION platform.notify_drops();
```

### 1.3 Payload Design Best Practices

| Concern | Recommendation |
|---------|--------------|
| **Size limit** | Stay under 8,000 bytes. Send only `event_id` + essential metadata in NOTIFY; fetch full payload from the outbox table. |
| **Format** | Use JSON/JSONB. PostgreSQL 9+ supports JSON payloads natively. |
| **Idempotency** | Include `event_id` (UUID) in every payload so consumers can deduplicate. |
| **Versioning** | Add a `version` field (e.g., `"version": "2024-06-01"`) to the payload schema for backward compatibility. |
| **Timestamping** | Use `timestamptz` for `created_at` and `processed_at` to maintain ordering across time zones. |
| **Schema context** | Include `schema_name` and `object_type` so consumers can filter for their domain. |

### 1.4 Avoiding Notification Flooding

DDL-heavy operations (e.g., schema migrations, bulk index creation) can generate hundreds of notifications. Mitigation strategies:

1. **Debounce in the trigger** — Use a `platform.ddl_batch` staging table and a `pg_sleep` loop or a `pg_cron` job to coalesce rapid DDL events into a single notification.
2. **Filter by object type** — Only listen for object types your consumers care about (tables, indexes, functions, not every possible DDL tag).
3. **Batch fetch from outbox** — Workers should fetch `LIMIT N` unprocessed rows per poll cycle rather than processing one-by-one.
4. **Rate-limit NOTIFY** — For known migration windows, temporarily disable the event trigger or route to a low-priority channel.

```sql
-- Debounce helper: coalesce recent DDL into one notification
CREATE OR REPLACE FUNCTION platform.debounced_notify()
RETURNS event_trigger AS $$
BEGIN
    -- Insert into batch table instead of immediate NOTIFY
    INSERT INTO platform.ddl_batch (event_data)
    SELECT jsonb_build_object(
        'command', command_tag,
        'object_identity', object_identity
    ) FROM pg_event_trigger_ddl_commands();
END;
$$ LANGUAGE plpgsql;
```

### 1.5 Propagation to External Clients (WebSocket, SSE, gRPC)

PostgreSQL LISTEN/NOTIFY is server-local. A bridge layer is required to push to external clients.

#### WebSocket Bridge (Python/FastAPI)

```python
from contextlib import asynccontextmanager
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
import asyncpg
import asyncio
import json

clients: List[WebSocket] = []

async def db_listener():
    conn = await asyncpg.connect("postgresql://user:pass@localhost/mydb")

    async def on_notification(conn, pid, channel, payload):
        event_id = json.loads(payload)
        # Fetch full event from outbox
        row = await conn.fetchrow(
            """UPDATE platform.schema_changes
               SET processed = true, processed_at = now()
               WHERE event_data->>'event_id' = $1 AND NOT processed
               RETURNING event_data""",
            event_id
        )
        if row:
            message = json.dumps({
                "type": "schema_change",
                "channel": channel,
                "data": row["event_data"]
            })
            disconnected = []
            for client in clients:
                try:
                    await client.send_text(message)
                except:
                    disconnected.append(client)
            for client in disconnected:
                clients.remove(client)

    await conn.add_listener('schema_changed', on_notification)
    try:
        while True:
            await asyncio.sleep(1)
    finally:
        await conn.remove_listener('schema_changed', on_notification)
        await conn.close()

@asynccontextmanager
async def lifespan(app: FastAPI):
    listener_task = asyncio.create_task(db_listener())
    try:
        yield
    finally:
        listener_task.cancel()
        try:
            await listener_task
        except asyncio.CancelledError:
            pass

app = FastAPI(lifespan=lifespan)

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    clients.append(websocket)
    try:
        while True:
            await websocket.receive_text()
    except WebSocketDisconnect:
        clients.remove(websocket)
```

#### SSE Bridge (Rust/Axum)

```rust
use axum::{
    response::Sse,
    routing::get,
    Router,
};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone)]
struct SchemaChangeEvent {
    event_id: String,
    command: String,
    object_identity: String,
}

type SchemaChangeTx = broadcast::Sender<SchemaChangeEvent>;

async fn sse_handler(tx: SchemaChangeTx) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let rx = tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| async move {
        match result {
            Ok(event) => Some(Ok(axum::response::sse::Event::default()
                .event("schema_change")
                .data(serde_json::to_string(&event).unwrap()))),
            Err(_) => None,
        }
    });
    Sse::new(stream)
}
```

#### gRPC Bridge Pattern

For gRPC, use a bi-directional streaming service or server-streaming RPC. The bridge consumes PostgreSQL notifications and pushes `SchemaChangeEvent` protobufs to subscribed gRPC clients. Use a `tokio::sync::broadcast` or `tokio::sync::mpsc` channel to fan-out from the single LISTEN connection to multiple gRPC streams.

### 1.6 Reliability Checklist

| Pattern | Implementation |
|---------|---------------|
| **At-least-once delivery** | Outbox table + `processed` flag + `UPDATE ... RETURNING` in the worker |
| **Exactly-once processing** | `event_id` deduplication in the consumer (idempotent consumers) |
| **Listener crash recovery** | Poll the outbox table on startup (`SELECT * FROM schema_changes WHERE NOT processed`) before starting LISTEN |
| **Connection pooling** | Use a dedicated connection for LISTEN (not pooled). Pooled connections may return to the pool and miss notifications. |
| **Monitoring** | Query `pg_listening_channels()` and `pg_notification_queue_usage()` to detect listener health and queue pressure. |

---

## 2. AG-UI and A2UI (AI-Generated UI) Patterns

### 2.1 What is A2UI?

A2UI (Agent-to-UI) is an open-source protocol (Google, Apache 2.0) that lets AI agents send **declarative UI descriptions** — JSON messages describing buttons, forms, charts, maps, dashboards — to a client, which renders them as native interactive components. It is transport-agnostic and works over A2A, AG-UI, MCP, SSE, WebSockets, or REST.

The core insight: agents describe **intent** (what UI to show), and the client renders it using its own native component library (React, Angular, Flutter, shadcn/ui, etc.). This avoids the security and UX problems of shipping raw HTML/JS from an untrusted agent.

### 2.2 The Three-Message Pattern

A2UI defines a minimal message protocol with four message types:

1. **`createSurface`** — Declares a new UI surface (a distinct, controllable region of the client's UI) with a `surfaceId` and `catalogId`.
2. **`updateComponents`** — Describes the component tree to render. Components are flat (not nested JSON) with `id` references to avoid LLM nesting errors.
3. **`updateDataModel`** — Provides the data/state to populate components. Data is bound via JSON Pointer paths (`/passenger/firstName`).
4. **`deleteSurface`** — Cleans up a surface when no longer needed.

```json
{
  "version": "v0.9",
  "createSurface": {
    "surfaceId": "booking-form",
    "catalogId": "https://a2ui.org/specification/v0_9/catalogs/basic/catalog.json"
  }
}
```

```json
{
  "version": "v0.9",
  "updateComponents": {
    "surfaceId": "booking-form",
    "components": [
      { "id": "root", "component": "Card", "child": "content" },
      { "id": "content", "component": "Column", "children": ["date", "time", "submit"] },
      { "id": "date", "component": "DateTimeInput", "label": "Date" },
      { "id": "time", "component": "ChoicePicker", "options": ["Morning", "Afternoon", "Evening"] },
      { "id": "submit", "component": "Button", "text": "Book", "action": { "event": { "name": "book" } } }
    ]
  }
}
```

```json
{
  "version": "v0.9",
  "updateDataModel": {
    "surfaceId": "booking-form",
    "path": "/booking",
    "value": { "date": "2026-07-01", "time": "Morning" }
  }
}
```

### 2.3 Metadata Hints for AI-Generated UI

For an AI agent to generate useful UI from metadata, the schema metadata must expose rich semantic hints. This is the bridge between database schema (or domain model) and dynamic UI generation.

#### Required Metadata Fields

| Category | Field | Description | Example |
|----------|-------|-------------|---------|
| **Field Types** | `type` | Scalar type | `string`, `integer`, `boolean`, `date`, `datetime`, `decimal`, `jsonb`, `uuid`, `enum` |
| | `format` | Display format | `email`, `uri`, `password`, `markdown`, `color`, `currency` |
| **Constraints** | `required` | Non-null | `true` |
| | `minLength` / `maxLength` | String bounds | `minLength: 3` |
| | `minimum` / `maximum` | Numeric bounds | `minimum: 0` |
| | `pattern` | Regex validation | `"^\\d{5}$"` |
| **Validation Rules** | `validator` | Named validation function | `isValidISBN`, `isFutureDate` |
| | `customError` | Error message for rule | `"Must be a valid ISBN-13"` |
| **Display** | `displayName` | Human-readable label | `"Customer Email"` |
| | `description` | Help text / tooltip | `"Used for order confirmations"` |
| | `placeholder` | Input placeholder | `"name@example.com"` |
| | `componentHint` | Preferred UI component | `TextField`, `DateTimeInput`, `ChoicePicker`, `Slider`, `Markdown`, `RichText` |
| | `visibility` | When to show | `always`, `create`, `edit`, `readonly` |
| **Relationships** | `relation` | Related entity | `{ "type": "belongsTo", "target": "users", "displayField": "email" }` |
| | `relationType` | Cardinality | `oneToOne`, `oneToMany`, `manyToMany` |
| | `nestedForm` | Inline vs modal | `inline`, `modal`, `drawer` |
| **Permissions** | `roles` | Who can see/edit | `["admin", "editor"]` |
| | `scope` | Access scope | `read`, `write`, `admin` |
| **Layout** | `group` | Form section | `"Contact Information"` |
| | `order` | Display order | `1` |
| | `columnSpan` | Grid layout | `1`, `2`, `full` |

#### Example: Schema Metadata JSON for a "Product" Entity

```json
{
  "entity": "product",
  "version": "2026-06-01",
  "displayName": "Product",
  "description": "A sellable item in the catalog",
  "permissions": {
    "read": ["viewer", "editor", "admin"],
    "write": ["editor", "admin"],
    "delete": ["admin"]
  },
  "fields": [
    {
      "name": "id",
      "type": "uuid",
      "required": true,
      "visibility": "readonly",
      "componentHint": "TextField",
      "displayName": "ID",
      "order": 0
    },
    {
      "name": "name",
      "type": "string",
      "required": true,
      "minLength": 2,
      "maxLength": 200,
      "displayName": "Product Name",
      "placeholder": "Enter product name",
      "componentHint": "TextField",
      "order": 1,
      "columnSpan": 2
    },
    {
      "name": "price",
      "type": "decimal",
      "required": true,
      "minimum": 0,
      "displayName": "Price",
      "format": "currency",
      "currency": "USD",
      "componentHint": "TextField",
      "order": 2
    },
    {
      "name": "status",
      "type": "enum",
      "options": ["draft", "published", "archived"],
      "required": true,
      "displayName": "Status",
      "componentHint": "ChoicePicker",
      "order": 3
    },
    {
      "name": "description",
      "type": "string",
      "format": "markdown",
      "maxLength": 5000,
      "displayName": "Description",
      "componentHint": "Markdown",
      "order": 4,
      "columnSpan": 2
    },
    {
      "name": "tags",
      "type": "string",
      "array": true,
      "displayName": "Tags",
      "componentHint": "ChoicePicker",
      "relation": { "type": "manyToMany", "target": "tag", "displayField": "name" },
      "order": 5
    },
    {
      "name": "createdAt",
      "type": "datetime",
      "required": true,
      "visibility": "readonly",
      "displayName": "Created At",
      "format": "datetime",
      "componentHint": "TextField",
      "order": 6
    }
  ],
  "layout": {
    "groups": [
      { "name": "Basic Info", "fields": ["name", "price", "status"], "order": 1 },
      { "name": "Content", "fields": ["description", "tags"], "order": 2 },
      { "name": "Metadata", "fields": ["id", "createdAt"], "order": 3 }
    ]
  }
}
```

### 2.4 Filtering Metadata by Identity / Permissions

Not all agents or users should see all metadata. The metadata layer must filter fields, relations, and even entire entities based on the requester's identity.

#### Filtering Strategy

1. **Role-Based Field Filtering** — Strip fields the agent/user has no `read` permission for before serializing metadata.
2. **Scope-Based Entity Visibility** — Exclude entire entities from the catalog if the agent lacks `read` scope.
3. **Redaction** — Replace sensitive values with `***` or omit them, rather than exposing existence.
4. **Dynamic Component Hints** — Downgrade `write` fields to `readonly` for viewers; hide `admin` fields from non-admins.

```python
# Example: Python metadata filter
from typing import List, Dict, Any

def filter_metadata_for_agent(
    metadata: Dict[str, Any],
    agent_roles: List[str]
) -> Dict[str, Any]:
    """Filter schema metadata based on agent identity/roles."""
    filtered = {**metadata}
    allowed_fields = []

    for field in metadata.get("fields", []):
        read_roles = field.get("permissions", {}).get("read", metadata.get("permissions", {}).get("read", []))
        if any(role in read_roles for role in agent_roles):
            # Determine effective visibility
            if any(role in field.get("permissions", {}).get("write", []) for role in agent_roles):
                field["visibility"] = field.get("visibility", "always")
            else:
                field["visibility"] = "readonly"
            allowed_fields.append(field)

    filtered["fields"] = allowed_fields
    return filtered
```

### 2.5 Transport Bindings and Real-Time Updates

A2UI is transport-agnostic. For real-time schema propagation (when the database schema changes and the UI must regenerate), the recommended transports are:

| Transport | Use Case | Pattern |
|-----------|----------|---------|
| **AG-UI** | Agent-to-user UI, streaming | Native binding in agent frameworks. Surfaces update incrementally. |
| **A2A** | Agent-to-agent UI exchange | A2UI messages are carried as `DataPart` with `mimeType: application/json+a2ui`. |
| **SSE** | Web clients, one-way updates | Server pushes `updateComponents` + `updateDataModel` messages to the browser. |
| **WebSockets** | Bi-directional, high interactivity | Client sends `action` messages (button clicks, form submits); server sends UI updates. |
| **gRPC** | Internal services, high throughput | Proto-wrapped A2UI JSON for inter-service UI generation. |

### 2.6 Architectural Patterns for AI-Generated UI

#### Pattern A: Metadata-Driven Form Generation

The agent receives entity metadata (from the database schema or a metadata registry), decides which fields to show, and generates an A2UI `updateComponents` message. The client renders it natively.

**Agent prompt (internal):**
```
You are a UI generator. Given the following metadata for a "Product" entity,
generate an A2UI updateComponents message for a create-form surface.
Rules:
- Show only fields with visibility != "hidden"
- Group fields by the "layout.groups" metadata
- Use ChoicePicker for enums, DateTimeInput for dates, Markdown for markdown fields
- Include a "Submit" button with action event "create_product"
```

#### Pattern B: Chat-to-Dashboard

A user asks an analytics agent: "Show me Q3 revenue by region." The agent queries the database, generates an A2UI surface with a `Column` layout containing a `Text` summary and a chart component (if the catalog supports it), and streams it via SSE to the browser.

#### Pattern C: Permission-Aware Admin Panels

An admin agent queries the schema metadata, filters it by the requesting admin's roles, and generates an editable table surface. Non-admin users get a `readonly` view with fewer columns and no action buttons.

### 2.7 Security Considerations

| Concern | Mitigation |
|---------|------------|
| **Schema exposure** | Never expose raw database schema. Use an explicit metadata layer with field-level permissions. |
| **Action injection** | Validate all `action` event names against an allowlist before executing backend logic. |
| **Data leakage** | Filter `updateDataModel` payloads by the same roles used for metadata filtering. |
| **Catalog trust** | The client advertises `supportedCatalogIds`. The agent must only use pre-approved catalogs. |
| **Validation** | Use the A2UI four-layer validator: JSON Schema, integrity checks, topology checks, recursion limits. |

---

## 3. ArcSwap and Atomic State Management in Rust

### 3.1 The Problem

In async Rust web services (Axum, Actix-web), shared state is typically passed via `Arc<AppState>`. But `Arc` is immutable. If you need to update configuration, reload certificates, or swap a router at runtime, you have three options:

1. **`Arc<RwLock<T>>`** — Writers block readers; contention under load.
2. **`tokio::sync::RwLock<T>`** — Async-compatible but still contended; holding across `.await` is tricky.
3. **`ArcSwap<T>`** — Lock-free, wait-free reads. Writers atomically swap the pointer. Old readers continue with the old `Arc`; new readers get the new one. Zero contention for reads.

### 3.2 ArcSwap Basics

`arc-swap` provides atomic operations on `Arc` pointers. Readers get a `Guard` (a smart pointer to the current `Arc`) without any locking. Writers atomically swap in a new `Arc`. The old `Arc` is dropped only when the last reader releases it.

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Config {
    pub server_port: u16,
    pub timeout_ms: u64,
    pub max_connections: usize,
    pub feature_flags: FeatureFlags,
}

#[derive(Clone, Debug)]
pub struct FeatureFlags {
    pub enable_caching: bool,
    pub enable_rate_limiting: bool,
    pub rate_limit_per_second: u32,
}

pub struct ConfigManager {
    config: ArcSwap<Config>,
}

impl ConfigManager {
    pub fn new(config: Config) -> Self {
        Self {
            config: ArcSwap::from_pointee(config),
        }
    }

    /// Lock-free read. Returns a Guard that derefs to Arc<Config>.
    pub fn get(&self) -> arc_swap::Guard<Arc<Config>> {
        self.config.load()
    }

    /// Atomic swap. Readers see either old or new, never partial.
    pub fn reload(&self, new_config: Config) {
        self.config.store(Arc::new(new_config));
    }
}
```

### 3.3 Validation Before Swap

Never swap in invalid configuration. The old configuration stays active if validation fails.

```rust
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.server_port == 0 {
            return Err("server_port must be non-zero".into());
        }
        if self.timeout_ms == 0 {
            return Err("timeout_ms must be non-zero".into());
        }
        if self.max_connections == 0 {
            return Err("max_connections must be non-zero".into());
        }
        if self.feature_flags.enable_rate_limiting
            && self.feature_flags.rate_limit_per_second == 0
        {
            return Err("rate_limit_per_second must be set when rate limiting is enabled".into());
        }
        Ok(())
    }
}

impl ConfigManager {
    pub fn reload_with_validation(&self, new_config: Config) -> Result<(), String> {
        new_config.validate()?;
        self.config.store(Arc::new(new_config));
        Ok(())
    }
}
```

### 3.4 Using ArcSwap in Axum Application State

```rust
use axum::{
    extract::State,
    routing::get,
    Router,
};
use std::sync::Arc;
use arc_swap::ArcSwap;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ConfigManager>,
    // ... other shared state
}

async fn handler(State(state): State<Arc<AppState>>) -> String {
    // Lock-free read of current configuration
    let config = state.config.get();
    format!("Timeout is {} ms", config.timeout_ms)
}

fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(handler))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let config = Config {
        server_port: 8080,
        timeout_ms: 5000,
        max_connections: 1000,
        feature_flags: FeatureFlags {
            enable_caching: true,
            enable_rate_limiting: false,
            rate_limit_per_second: 0,
        },
    };

    let config_manager = Arc::new(ConfigManager::new(config));
    let state = Arc::new(AppState {
        config: config_manager.clone(),
    });

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    // Spawn a background task that reloads config from a file or database
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            // In reality, load from file / PostgreSQL / remote config service
            if let Ok(new_config) = load_config_from_disk().await {
                let _ = config_manager.reload_with_validation(new_config);
            }
        }
    });

    axum::serve(listener, app).await.unwrap();
}
```

### 3.5 Hot-Swappable Router in Axum

A more advanced pattern: swap the entire Axum router at runtime without dropping connections. This is useful when routes change based on feature flags or schema changes.

```rust
use arc_swap::ArcSwap;
use axum::{
    body::Body,
    extract::Request,
    response::Response,
    Router,
};
use std::sync::Arc;
use tower::Service;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;

/// A tower Service that wraps an ArcSwap<Router>, allowing atomic router swaps.
pub struct SwappableRouter {
    router: ArcSwap<Router>,
}

impl SwappableRouter {
    pub fn new(router: Router) -> Self {
        Self {
            router: ArcSwap::new(Arc::new(router)),
        }
    }

    pub fn swap(&self, new_router: Router) {
        self.router.store(Arc::new(new_router));
    }
}

impl Clone for SwappableRouter {
    fn clone(&self) -> Self {
        Self {
            router: ArcSwap::new(self.router.load().clone()),
        }
    }
}

impl Service<Request<Body>> for SwappableRouter {
    type Response = Response<Body>;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let router = self.router.load().clone();
        Box::pin(async move {
            match router.oneshot(req).await {
                Ok(res) => Ok(res),
                Err(_) => Ok(Response::builder()
                    .status(500)
                    .body(Body::from("Router error"))
                    .unwrap()),
            }
        })
    }
}

// Usage
#[tokio::main]
async fn main() {
    let router_v1 = Router::new().route("/", get(|| async { "v1" }));
    let swappable = SwappableRouter::new(router_v1);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    // Later, when schema changes or feature flags toggle:
    let router_v2 = Router::new()
        .route("/", get(|| async { "v2" }))
        .route("/new", get(|| async { "new endpoint" }));
    swappable.swap(router_v2);

    axum::serve(listener, swappable).await.unwrap();
}
```

**Note:** `axum::serve` in newer versions (0.7+) expects a `Router` directly. The `SwappableRouter` pattern above is best used with a custom `tower::Service` wrapper or by restarting the `axum::serve` task with a graceful shutdown signal. A simpler, production-ready approach:

```rust
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    let (router_tx, router_rx) = watch::channel(Router::new().route("/", get(|| async { "v1" })));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    // Spawn the server task
    let server_task = tokio::spawn(async move {
        let mut router_rx = router_rx;
        loop {
            let router = router_rx.borrow_and_update().clone();
            // In axum 0.7+, we need a graceful shutdown to swap routers
            // This is a simplified sketch; production code uses with_graceful_shutdown
            let serve = axum::serve(listener, router);
            // ... handle shutdown and rebind
        }
    });

    // When a new router is needed:
    let new_router = Router::new().route("/", get(|| async { "v2" }));
    let _ = router_tx.send(new_router);
}
```

### 3.6 `tokio::sync::watch` for Async State Broadcast

`tokio::sync::watch` is ideal for broadcasting a single value to many receivers. It is not a queue — it holds the latest value. Perfect for "current configuration" or "current router."

```rust
use tokio::sync::watch;
use std::sync::Arc;

#[derive(Clone)]
pub struct SharedState {
    pub config: Arc<Config>,
    pub router: Router,
}

pub type StateWatch = watch::Sender<SharedState>;

async fn background_reload(tx: StateWatch) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    loop {
        interval.tick().await;
        // Load new config
        let new_config = load_config().await;
        let new_router = build_router(new_config.clone());
        let _ = tx.send(SharedState {
            config: Arc::new(new_config),
            router: new_router,
        });
    }
}
```

### 3.7 `parking_lot` for Fast Synchronization

`parking_lot` provides smaller, faster mutexes and RwLocks than the standard library. It is useful when you need a small amount of real locking (e.g., a callback registry) alongside your lock-free reads.

```rust
use parking_lot::Mutex;
use std::sync::Arc;

pub struct ConfigManagerWithCallbacks {
    config: ArcSwap<Config>,
    callbacks: Mutex<Vec<Box<dyn Fn(&Config, &Config) + Send + Sync>>>,
}

impl ConfigManagerWithCallbacks {
    pub fn on_change<F>(&self, callback: F)
    where
        F: Fn(&Config, &Config) + Send + Sync + 'static,
    {
        self.callbacks.lock().push(Box::new(callback));
    }

    pub fn reload(&self, new_config: Config) -> Result<(), String> {
        new_config.validate()?;
        let old = self.config.get().clone();
        self.config.store(Arc::new(new_config));
        let new = self.config.get().clone();
        let callbacks = self.callbacks.lock();
        for cb in callbacks.iter() {
            cb(&old, &new);
        }
        Ok(())
    }
}
```

### 3.8 Production Pattern: Certificate Hot-Reload (rsigma-style)

The `rsigma` daemon demonstrates a real-world pattern: TLS certificates are re-read from disk and atomically swapped via `Arc<ArcSwap<rustls::ServerConfig>>`.

Key characteristics:
- **Debounced reload** — File watcher, SIGHUP, and HTTP POST all funnel into a single reload task.
- **Validation** — New cert/key are parsed before swapping.
- **Graceful failure** — If reload fails, the previous certificate stays active; metrics (`rsigma_reloads_failed_total`) are incremented.
- **Zero connection drop** — In-flight TLS connections are not dropped; only new connections use the new certificate.

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;
use rustls::ServerConfig;

pub struct TlsManager {
    config: Arc<ArcSwap<ServerConfig>>,
}

impl TlsManager {
    pub fn get(&self) -> arc_swap::Guard<Arc<ServerConfig>> {
        self.config.load()
    }

    pub fn reload(&self, cert_path: &str, key_path: &str) -> Result<(), String> {
        let new_config = load_tls_config(cert_path, key_path)?;
        self.config.store(Arc::new(new_config));
        Ok(())
    }
}
```

### 3.9 Comparison Matrix

| Primitive | Use Case | Lock-Free | Async-Aware | Latency |
|-----------|----------|-----------|-------------|---------|
| `ArcSwap<T>` | Hot-reload config, router, TLS certs | Yes | No (blocking read, but extremely fast) | Nanoseconds |
| `tokio::sync::watch` | Broadcast current state to many async tasks | Yes (for receives) | Yes | Microseconds |
| `parking_lot::Mutex` | Short critical sections, callback registries | No | No (blocking) | Sub-microsecond |
| `parking_lot::RwLock` | Read-heavy, infrequent writes | No | No (blocking) | Sub-microsecond read |
| `tokio::sync::RwLock` | Async read/write locks | No | Yes | Higher overhead |

### 3.10 Architectural Recommendations

1. **Prefer `ArcSwap` for state that is read thousands of times per second and written infrequently.** It is the fastest option for lock-free reads.
2. **Use `tokio::sync::watch` for async tasks that need to react to state changes.** Combine with `ArcSwap` by storing the `ArcSwap` inside the watch channel's value.
3. **Use `parking_lot` for real locking when you need mutation (e.g., a counter, a registry).** It is faster and more compact than `std::sync::Mutex`.
4. **Never hold a lock across an `.await` point unless it is `tokio::sync::Mutex` or `tokio::sync::RwLock`.** `parking_lot` and `std::sync` locks are not async-aware.
5. **Validate before swap.** An invalid configuration must never replace a working one.
6. **Use callbacks or watch channels for secondary effects.** When config reloads, you may need to resize a connection pool or clear a cache. Do this after the atomic swap, not inside it.

---

## 4. Integration: Real-Time Schema → AI-Generated UI → Hot Router Swap

This section ties the three research areas together into a coherent architecture.

### 4.1 End-to-End Flow

```
┌─────────────────┐     DDL event      ┌──────────────────────┐
│  PostgreSQL     │ ─────────────────> │  Rust Listener       │
│  (Event Trigger)│   LISTEN/NOTIFY    │  (tokio-postgres)  │
└─────────────────┘                    └──────────────────────┘
                                              │
                                              │ fetch outbox
                                              │
                                              v
┌─────────────────┐     JSON event     ┌──────────────────────┐
│  Metadata Registry│ <──────────────── │  Schema Change       │
│  (ArcSwap<Meta>)│   update + notify   │  Processor           │
└─────────────────┘                    └──────────────────────┘
                                              │
                                              │ ArcSwap swap
                                              v
┌─────────────────┐   A2UI messages    ┌──────────────────────┐
│  AI Agent (LLM) │ <──────────────── │  UI Generator        │
│                 │   metadata diff    │  (tokio::sync::watch) │
└─────────────────┘                    └──────────────────────┘
                                              │
                                              │ generate/update
                                              v
┌─────────────────┐   A2UI JSON        ┌──────────────────────┐
│  Web Client     │ <──────────────── │  SSE / WebSocket     │
│  (React/shadcn) │   updateComponents │  Bridge              │
└─────────────────┘                    └──────────────────────┘
                                              │
                                              │ hot swap
                                              v
┌─────────────────┐                    ┌──────────────────────┐
│  Axum Server    │ <───────────────── │  Swappable Router    │
│  (Router)       │   new routes       │  (ArcSwap<Router>)  │
└─────────────────┘                    └──────────────────────┘
```

### 4.2 Key Integration Points

1. **PostgreSQL → Rust Listener** — A dedicated `tokio-postgres` connection runs `LISTEN schema_changed;`. On notification, it fetches the full event from the outbox and updates the `ArcSwap<MetadataRegistry>`.

2. **Metadata Registry → AI Agent** — The registry holds a `tokio::sync::watch::Sender` that broadcasts metadata changes to the agent subsystem. The agent re-generates A2UI surfaces when the metadata for an entity changes.

3. **A2UI → Client** — The generated A2UI messages are sent to connected clients via SSE or WebSocket. The client renders new forms, tables, or dashboards on-the-fly.

4. **Schema Change → Router Swap** — If a new entity is created, the `SwappableRouter` is updated to include new REST/gRPC endpoints. Existing connections are not dropped.

### 4.3 Example: Combined Rust Application State

```rust
use arc_swap::ArcSwap;
use axum::Router;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Clone)]
pub struct MetadataRegistry {
    pub entities: Vec<EntityMetadata>,
    pub catalogs: Vec<CatalogDefinition>,
}

#[derive(Clone)]
pub struct AppState {
    pub metadata: Arc<ArcSwap<MetadataRegistry>>,
    pub metadata_watch: watch::Sender<MetadataRegistry>,
    pub router: Arc<ArcSwap<Router>>,
    pub config: Arc<ConfigManager>,
}

impl AppState {
    pub fn update_metadata(&self, new_registry: MetadataRegistry) {
        // 1. Atomic swap of metadata
        self.metadata.store(Arc::new(new_registry.clone()));
        // 2. Notify async watchers (AI agents, SSE clients)
        let _ = self.metadata_watch.send(new_registry.clone());
        // 3. Rebuild and swap router if endpoints changed
        let new_router = build_router(new_registry);
        self.router.store(Arc::new(new_router));
    }
}
```

---

## 5. References and Sources

1. **PostgreSQL LISTEN/NOTIFY**
   - Pedro Alonso, "PostgreSQL LISTEN/NOTIFY: Real-Time Without the Message Broker" (2025-11-03)
   - OneUptime, "How to Use Listen/Notify for Real-Time Updates in PostgreSQL" (2026-01-25)
   - Neon, "Postgres as Your Platform: Building Event-Driven Systems with Schema Changes" (2025-07-25)
   - OneUptime, "How to Implement PostgreSQL Event Triggers" (2026-01-30)

2. **A2UI / AG-UI**
   - A2UI Specification, https://a2ui.org/specification/v0.9-a2ui/
   - A2UI v1.0 Specification, https://a2ui.org/specification/v1.0-a2ui/
   - Google A2UI Extension Spec, https://github.com/google/A2UI
   - Angular Architects, "A2UI: How AI Generates Dynamic UIs at Runtime" (2026-05-13)
   - ChartGen, "From Chatbot to Dashboard: How Google's A2UI Protocol Is Redefining What AI Agents Can Show You" (2026-05-12)
   - Gentic News, "Google Launches A2UI 0.9, a Generative UI Standard for AI Agents" (2026-04-19)
   - coolxeo/a2ui-adk GitHub repository (2026-03-07)

3. **Rust Atomic State / ArcSwap**
   - OneUptime, "How to Implement Hot Configuration Reloading in Rust" (2026-01-25)
   - Apáti Sándor, "Production-ready microservice in Rust: 5. Application State" (2023-05-29)
   - rust-api.dev, "Understand Axum" (2021-05-01)
   - tokio.rs, "What's new in axum 0.6.0-rc.1" (2022-08-23)
   - conf-hub crate, https://lib.rs/crates/conf-hub (2026-03-22)
   - timescale/rsigma changelog, v0.13.0 cert hot-reload (2026-05-26)
   - axum docs, `State` extractor: https://docs.rs/axum/latest/axum/extract/struct.State.html
   - tokio-rs/axum GitHub discussion #2752, "how to achieve immutable shared state?" (2024-05-27)
