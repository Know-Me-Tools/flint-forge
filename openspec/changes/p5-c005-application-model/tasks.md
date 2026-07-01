# p5-c005 Tasks — Application Model

## Tasks

- [ ] Add `flint_a2ui.resolve_components(application_id, jwt_claims)` SECURITY DEFINER function
- [ ] Implement role hierarchy resolution (parent role → child role inheritance lookup)
- [ ] Add Cedar capability definitions: `a2ui:view`, `a2ui:register`, `a2ui:emit` in `forge-policy`
- [ ] Gate test: `resolve_components()` returns base components for anonymous user
- [ ] Gate test: `resolve_components()` returns app-specific components only for users with role assignments
- [ ] Gate test: user with no role in app sees only base components (not app-specific ones)
