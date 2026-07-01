# p5-c007 Tasks — Event-Driven Assembly

## Tasks

- [ ] Add `fdb-reflection/src/compilers/a2ui.rs` with `A2uiAssembler`, `AssemblyContext`, `A2uiSurface`, `A2uiMessage` types
- [ ] Implement `A2uiAssembler::assemble()`: assembly rules lookup → component resolution → message construction
- [ ] Add default assembly path: if no rule matches, use `flint_a2ui.bindings` for the event's source table → grid component
- [ ] Add `AssemblerError` thiserror enum (no unwrap/expect in lib code)
- [ ] Wire assembler to `POST /a2ui/v1/surfaces/assemble` endpoint (p5-c006)
- [ ] Iggy integration: if FRF Phase 3 Iggy producer is available, push assembled surfaces to `a2ui.surfaces` topic; else return synchronously
- [ ] Gate test: `assemble()` for a `tool_call_completed` event with `data_source = public.orders` returns an `updateComponents` message containing a `DataGrid` component
- [ ] Gate test: assembly completes in < 500ms for a single surface
- [ ] Gate test: no assembly rule match → falls back to default grid binding
