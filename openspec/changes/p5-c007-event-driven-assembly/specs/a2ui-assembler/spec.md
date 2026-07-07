## ADDED Requirements

### Requirement: Assembler exposes a typed Rust API
The system SHALL provide `A2uiAssembler`, `AssemblyContext`, `A2uiSurface`, and `A2uiMessage` types in `fdb-reflection/src/compilers/a2ui.rs`.

#### Scenario: Construct assembler from a PgPool
- **WHEN** the gateway calls `A2uiAssembler::new(pool)`
- **THEN** it receives an assembler instance that can be shared across requests

#### Scenario: Assemble context carries all required inputs
- **WHEN** the caller builds an `AssemblyContext` with `event_type`, `event_payload`, `application_id`, `jwt_claims`, and optional `surface_id`
- **THEN** the assembler accepts the context without requiring additional side channels

### Requirement: Assembler emits valid A2UI v0.9.1 messages
The system SHALL produce a message sequence containing `createSurface`, `updateComponents`, and `updateDataModel` messages for a single surface.

#### Scenario: Create surface declares target surface
- **WHEN** assembly succeeds
- **THEN** the first message is a `createSurface` with a stable `surfaceId` and a `catalogId` referencing the Flint catalog

#### Scenario: Update components describes the component tree
- **WHEN** assembly succeeds
- **THEN** the returned surface contains an `updateComponents` message with a flat list of components, each with a unique `id` and a valid `component` primitive type

#### Scenario: Update data model binds payload data
- **WHEN** assembly succeeds for an event with tabular data
- **THEN** the returned surface contains an `updateDataModel` message binding the data to a JSON Pointer path on the surface

### Requirement: Assembly rules take precedence over defaults
The system SHALL query `flint_a2ui.assembly_rules` for the application and event type, evaluate each rule's `event_filter` against the event payload in priority order, and use the first matching rule's `assembly_config`.

#### Scenario: Matching custom rule overrides default grid
- **GIVEN** an active rule for event type `tool_call_completed` with filter `{"data_source.table":"orders"}` and config selecting a `Card` component
- **WHEN** the assembler receives an event matching that filter
- **THEN** the resulting surface uses the `Card` component from the rule config, not the default `DataGrid`

#### Scenario: No matching rule falls back to default binding
- **GIVEN** no rule matches the event payload
- **WHEN** the event payload includes `data_source = {"schema":"public","table":"orders"}`
- **THEN** the assembler looks up `flint_a2ui.bindings` and emits a grid component bound to `public.orders`

### Requirement: Default binding path is deterministic
The system SHALL, when no rule matches, map a table binding to an A2UI component in the following order: `grid`, `form`, `detail`, `card`.

#### Scenario: Grid binding exists for table
- **GIVEN** a `grid` binding exists for `public.orders`
- **WHEN** no rule matches
- **THEN** the assembler emits a `DataGrid` component

#### Scenario: Only form binding exists for table
- **GIVEN** no `grid` binding exists for `public.orders` but a `form` binding does
- **WHEN** no rule matches
- **THEN** the assembler emits a `Form` component

### Requirement: Assembly errors are strongly typed
The system SHALL define an `AssemblerError` thiserror enum and SHALL NOT use `unwrap()` or `expect()` in library code paths.

#### Scenario: Missing binding returns typed error
- **WHEN** no rule matches and no binding exists for the event's source table
- **THEN** `assemble()` returns `AssemblerError::NoBinding`

#### Scenario: Database failure returns typed error
- **WHEN** a SQL query fails during assembly
- **THEN** `assemble()` returns `AssemblerError::Database`

### Requirement: Assembler is wired to the REST endpoint
The system SHALL replace the HTTP 501 stub in `POST /a2ui/v1/surfaces/assemble` with a call to `A2uiAssembler::assemble()`.

#### Scenario: Valid JWT receives assembled surface
- **GIVEN** a request to `/a2ui/v1/surfaces/assemble` with a valid JWT and a `tool_call_completed` event referencing `public.orders`
- **WHEN** the endpoint handler executes
- **THEN** it returns HTTP 200 with an A2UI surface JSON payload

#### Scenario: Unauthenticated request is rejected
- **GIVEN** a request without a valid JWT
- **WHEN** it reaches the `/a2ui/v1/surfaces/assemble` route
- **THEN** the `rls_layer::require_rls` middleware returns HTTP 401 before the assembler runs

### Requirement: Assembly meets latency SLA
The system SHALL complete single-surface assembly in less than 500ms under normal conditions.

#### Scenario: Single surface assembly timing
- **WHEN** the assembler processes a `tool_call_completed` event for a single bound table
- **THEN** the elapsed time from context acceptance to surface return is less than 500ms

### Requirement: Optional Iggy publisher integration
The system SHALL accept an optional `IggyPublisher` trait object and publish assembled surfaces to the `a2ui.surfaces` topic when the publisher is provided.

#### Scenario: Publisher present emits to topic
- **GIVEN** the gateway has injected an Iggy publisher into the assembler
- **WHEN** assembly succeeds
- **THEN** the surface is published to `a2ui.surfaces` in addition to being returned

#### Scenario: Publisher absent returns synchronously
- **GIVEN** no Iggy publisher is configured
- **WHEN** assembly succeeds
- **THEN** the surface is returned synchronously with no topic emission
