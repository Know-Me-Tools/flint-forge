# p0-c004 — Tasks (flint-realtime-fabric)

- [ ] Add WatchEntityTypeRequest + RPC to entity.proto; regenerate.
- [ ] SubscribePipeline: bind entity_type + filter; reuse EntityChange envelope.
- [ ] Keto coarse check at subscribe time (subject, view, entity_type|tenant).
- [ ] Integration test: two tenants, RLS-independent fabric filter correctness.
- [ ] GATE: proto frozen + stream verified; report; stop.
