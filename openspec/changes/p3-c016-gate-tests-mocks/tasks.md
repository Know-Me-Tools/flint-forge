# Tasks — p3-c016-gate-tests-mocks

- [ ] 1. Implement mock `ChangeStreamSource` emitting fixed `EntityChange` stream
- [ ] 2. Implement mock RLS re-query returning zero rows for unauthorized subscriber
- [ ] 3. Write `test_subscription_rls_drops_unauthorized_events` — assert zero delivered events
- [ ] 4. Write `test_keto_check_gates_mutation` — assert 403 on `MockKetoCheck::check() == false`
- [ ] 5. Positive-path assertion: keto allow → mutation reaches mock executor
- [ ] 6. `cargo test --workspace` green
