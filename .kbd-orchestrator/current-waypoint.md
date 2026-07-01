# Current Waypoint — Flint Forge

**Phase:** p3-auth-rls-keto
**Status:** plan_complete
**Updated:** 2026-07-01

## Next Action

```
/kbd-apply p3-c010-mount-reflection-router
```

## Phase 3 Change Order (9 changes)

1. `p3-c010-mount-reflection-router` — unblock REST testing
2. `p3-c011-ketocheck-port-trait` — G2 Keto coarse check port
3. `p3-c012-forge-policy-cedar` — G1 Cedar policy engine
4. `p3-c013-rest-handle-list` — G3 list handler + `is_safe_identifier()`
5. `p3-c014-rest-handle-mutations` — G3 insert/update/delete
6. `p3-c015-gate-tests-rest-and-vault` — G6 tests 1+2
7. `p3-c016-gate-tests-mocks` — G6 tests 3+4 (mocks)
8. `p3-c017-fdb-realtime-stub` — G7 production stub (OQ-FRF-1 conditional)
9. `p3-c018-introspection-merge-verify` — G4 confidence (OQ-3 conditional)

## Resolved Open Questions

- **OQ-cedar** — pin `cedar-policy = "4"` (current 4.11.2)
- **OQ-cedar-table** — `flint_meta.cedar_policies` absent; p3-c012 adds it

## Open Questions Carried

- OQ-3 — pg_graphql PG18 (gates p3-c018 scope)
- OQ-FRF-1 — FRF WatchEntityType RPC (gates p3-c017 live path; stub ships regardless)
- OQ-9 — pgvector PG18 image (CI wiring)
- OQ-12 — `flint_meta.agui_descriptor()` GRANT scope (gates p5-c009)
