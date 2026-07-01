# Tasks — p3-c013-rest-handle-list

- [x] 1. Locate or create `is_safe_identifier()`; cover with unit tests
- [x] 2. Define filter-operator parser (PostgREST-style `?col=eq.value`)
- [x] 3. Implement `handle_list` SELECT builder with parameterized values
- [x] 4. Wire `Range` header → LIMIT/OFFSET; emit `Content-Range`
- [x] 5. Call `is_safe_identifier()` on every column + table name
- [x] 6. Split `rest.rs` into directory module if approaching 500 lines
- [x] 7. Unit tests for each of the 12 operators
- [x] 8. `cargo check` + clippy + `cargo test -p fdb-reflection`
