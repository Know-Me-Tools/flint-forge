# p5-c009 Tasks — CompiledState Upgrade

## Tasks

- [x] Add `A2uiCatalog` and `A2uiCatalogEntry` structs to `fdb-reflection/src/compiled.rs`
- [x] Add `CompiledState.a2ui_catalog: Arc<A2uiCatalog>` field (note: there was no prior `agui_descriptors` field to rename — this is a new addition)
- [x] Add `ReflectionEngine::load_a2ui_catalog()` with graceful degradation when `flint_a2ui` schema is absent — engine.rs
- [x] Update `StateManager::do_compile()` to call `load_a2ui_catalog()` and populate `a2ui_catalog`
- [x] No callers of `agui_descriptors` existed (consumer audit confirmed) — n/a
- [ ] Cedar `a2ui:emit` capability check in `fke-server` WASM output handler — deferred: fke-server is stub phase; blocked on Cedar engine implementation
- [ ] `PolicyError::A2uiEmitDenied` variant — deferred: same blocker
- [x] Graceful degradation: when schema absent, `a2ui_catalog` is empty (`A2uiCatalog::empty()`)
- [x] `cargo check --workspace` passes
- [x] `A2uiCatalog` and `A2uiCatalogEntry` exported from `fdb-reflection/src/lib.rs`
- [x] `Debug` impl updated to include `a2ui_components` count

**Notes:**
- Cedar `a2ui:emit` and `PolicyError::A2uiEmitDenied` tasks blocked on Phase 7 Cedar engine (fke-server is todo!() stubs). Marked deferred — will be implemented in p7-c002 or p7-c007 when Cedar evaluation is wired.
