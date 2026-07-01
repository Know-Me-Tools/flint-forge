# Tasks — p3-c018-introspection-merge-verify

- [ ] 1. Run OQ-3 pre-flight: `SELECT extversion … pg_graphql` against PG18 container
- [ ] 2. Record result (version string or "absent") in proposal Verification Log
- [ ] 3. If present: implement/verify `IntrospectionMerger::merge()` union semantics
- [ ] 4. If present: unit test with two fixture SDLs asserting complete union
- [ ] 5. If absent: document 501 stub fallback in gateway
- [ ] 6. If absent: record `kbd_stage_handoff_skip` note
- [ ] 7. `cargo check` + clippy + relevant test
