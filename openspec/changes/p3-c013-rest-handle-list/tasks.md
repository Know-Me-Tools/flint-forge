# Tasks — p3-c013-rest-handle-list

- [ ] 1. Locate or create `is_safe_identifier()`; cover with unit tests
- [ ] 2. Define filter-operator parser (PostgREST-style `?col=eq.value`)
- [ ] 3. Implement `handle_list` SELECT builder with parameterized values
- [ ] 4. Wire `Range` header → LIMIT/OFFSET; emit `Content-Range`
- [ ] 5. Call `is_safe_identifier()` on every column + table name
- [ ] 6. Split `rest.rs` into directory module if approaching 500 lines
- [ ] 7. Unit tests for each of the 12 operators
- [ ] 8. `cargo check` + clippy + `cargo test -p fdb-reflection`
