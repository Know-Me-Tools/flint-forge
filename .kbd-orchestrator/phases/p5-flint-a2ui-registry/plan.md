# Phase 5 Plan — Flint A2UI Component Registry

## Order of attack
1. Verify p5-c001 gate tests pass (integration checkpoint).
2. Implement `p5-c004-embeddings-pipeline` — enables semantic/hybrid search used by p5-c006.
3. Implement `p5-c005-application-model` — enables component resolution and scoping.
4. Implement `p5-c006-rest-api` — exposes registry via HTTP.
5. Implement `p5-c007-event-driven-assembly` — critical path for Phase 7 A2UI emission.
6. Implement `p5-c008-protocol-surfaces` — A2A/MCP surfaces.
7. Implement `p5-c012-htmx-renderer` — server-side renderer path.
8. Implement `p5-c013-opendesign-integration` and `p5-c015-claude-design-skill` — design-tool integrations.

## Verification cadence
- Run `cargo check -p fdb-gateway` and `cargo check -p fdb-reflection` after each Rust change.
- Run targeted gate tests at integration checkpoints (max 3 waits).
- Final phase gate: `cargo test -p fdb-gateway` and `cargo test -p fdb-reflection`.

## Documentation
- Keep OpenSpec change `tasks.md` updated as tasks complete.
- Update `docs/FLINT-A2UI-REGISTRY-SPEC.md` if behavior diverges from spec.
