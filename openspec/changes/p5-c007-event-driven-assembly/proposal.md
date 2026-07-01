# p5-c007 — Event-Driven Component Assembly

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P1  
**Depends on:** p5-c001, p5-c003 (bindings), Phase 3 (FRF Iggy integration)  
**Blocks:** p7-c005 (A2UI surface emitter uses this assembler)

---

## What this change delivers

A Rust component assembler in `fdb-reflection` that takes an event (tool call completion, DB change, agent inference) and produces a valid A2UI v0.9.1 message sequence:
- `createSurface` → identifies the target surface and catalog
- `updateComponents` → constructs the component tree from `flint_a2ui.bindings` + `assembly_rules`
- `updateDataModel` → binds data to the component tree

### Assembler interface

```rust
// fdb-reflection/src/compilers/a2ui.rs

pub struct A2uiAssembler {
    pool: PgPool,
}

pub struct AssemblyContext {
    pub event_type: String,
    pub event_payload: serde_json::Value,
    pub application_id: Uuid,
    pub jwt_claims: serde_json::Value,
    pub surface_id: Option<String>,
}

pub struct A2uiSurface {
    pub surface_id: String,
    pub catalog_id: String,
    pub messages: Vec<A2uiMessage>,
}

pub enum A2uiMessage {
    CreateSurface(CreateSurface),
    UpdateComponents(UpdateComponents),
    UpdateDataModel(UpdateDataModel),
}

impl A2uiAssembler {
    pub async fn assemble(&self, ctx: AssemblyContext) -> Result<A2uiSurface, AssemblerError> {
        // 1. Look up matching assembly rules from flint_a2ui.assembly_rules
        // 2. If no rule matches, use default: table binding → grid component
        // 3. Resolve components from flint_a2ui.resolve_components()
        // 4. Construct A2UI message sequence
        // 5. Return surface for AG-UI Custom event emission (Phase 7)
    }
}
```

### SLA

Tool call completion → assembled surface: < 500ms. This requires the assembly rules lookup and component resolution to be fast (indexed lookups only; no N+1).

---

## Iggy integration (topic: a2ui.surfaces)

When an assembly is triggered by a database change event (not a direct API call), the assembled surface is pushed to the Iggy topic `a2ui.surfaces` for consumption by the realtime fabric. This requires FRF Phase 3 (Iggy producer). If FRF Phase 3 is not yet available, the assembler falls back to returning the surface synchronously via the REST API.
