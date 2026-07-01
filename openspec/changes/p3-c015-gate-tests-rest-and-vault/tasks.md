# Tasks — p3-c015-gate-tests-rest-and-vault

- [ ] 1. Locate `CompiledState` definition; confirm its serde representation
- [ ] 2. Write `test_vault_dek_not_in_compiled_state` — assert no DEK/vault_key field in JSON or Debug
- [ ] 3. Write `test_rest_select_with_eq_filter` skeleton — one test per filter operator
- [ ] 4. Add ≥6 SQL-injection-attempt cases asserting `is_safe_identifier()` rejects
- [ ] 5. Add round-trip assertion that generated SQL parameterizes values (not interpolates)
- [ ] 6. `cargo test --workspace` green
