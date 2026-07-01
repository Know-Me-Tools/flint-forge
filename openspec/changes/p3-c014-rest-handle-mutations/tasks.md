# Tasks — p3-c014-rest-handle-mutations

- [ ] 1. Implement `handle_insert` with RETURNING + 201 + Location
- [ ] 2. Implement `handle_update` with filter reuse + 200/204
- [ ] 3. Implement `handle_delete` with filter reuse + 204
- [ ] 4. Wire `KetoCheck::check()` + `Pep::check()` before each mutation
- [ ] 5. Map Keto/Cedar deny → typed 403
- [ ] 6. Reuse `is_safe_identifier()` for every identifier
- [ ] 7. Unit tests per handler (success, keto-deny, cedar-deny, injection-attempt)
- [ ] 8. `cargo check` + clippy + `cargo test -p fdb-reflection`
