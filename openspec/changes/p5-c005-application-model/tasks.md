# p5-c005 Tasks — Application Model

## Tasks

- [x] Add `flint_a2ui.resolve_components(application_id, jwt_claims)` SECURITY DEFINER function
- [x] Implement role hierarchy resolution (parent role → child role inheritance lookup)
- [x] Add Cedar capability definitions: `a2ui:view`, `a2ui:register`, `a2ui:emit` in `forge-policy`
- [x] Gate test: `resolve_components()` returns base components for anonymous user
- [x] Gate test: `resolve_components()` returns app-specific components only for users with role assignments
- [x] Gate test: user with no role in app sees only base components (not app-specific ones)

## Verification

```bash
# Compile-time checks (no DB required)
cargo clippy -p fdb-gateway -p forge-policy -- -D warnings
cargo test -p fdb-gateway --test a2ui_application_model_test   # skips cleanly without DATABASE_URL

# Full gate tests (requires Postgres)
DATABASE_URL=postgres://... cargo test -p fdb-gateway --test a2ui_application_model_test
```

## Notes

- Migration `0006_flint_a2ui_application_model.sql` adds:
  - `flint_a2ui.resolve_components(uuid, jsonb)` — base + permission-filtered app components
  - `flint_a2ui.resolve_role_descendants(uuid)` — recursive role inheritance
  - `flint_a2ui.resolve_application_roles(uuid, jsonb)` — effective roles for a user
  - Trigger guard preventing deletion of system applications
- `forge-policy/src/a2ui.rs` adds A2UI Cedar capability constants (`A2UI_VIEW`, `A2UI_REGISTER`, `A2UI_EMIT`) and the `A2UIPep` convenience trait.
- Tests create a temporary application, component, roles, and role assignments, then clean up.
