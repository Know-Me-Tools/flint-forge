# p5-c005 — Application Model and Permissions

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P1  
**Depends on:** p5-c001  
**Blocks:** p5-c006 (REST API needs permission-filtered queries), p5-c009 (CompiledState needs application-scoped catalog)

---

## What this change delivers

- Application CRUD endpoints (admin-only)
- Role hierarchy management
- `flint_a2ui.resolve_components(application_id uuid, jwt_claims jsonb)` — the canonical permission-filtered component query function
- JWT claims template resolution: the `jwt_claims_template` JSONB on `applications` defines which claims from the flint-gate JWT are mapped to which A2UI permission scopes
- Cedar policy integration: `a2ui:view`, `a2ui:register`, `a2ui:emit` capabilities

### resolve_components function

```sql
CREATE OR REPLACE FUNCTION flint_a2ui.resolve_components(
    p_application_id uuid,
    p_jwt_claims     jsonb
) RETURNS TABLE (
    id              uuid,
    slug            text,
    category        text,
    primitive_type  text,
    schema          jsonb,
    description     text
) LANGUAGE sql STABLE SECURITY DEFINER AS $$
    -- Base components: always visible
    SELECT id, slug, category, primitive_type, schema, description
    FROM flint_a2ui.components
    WHERE is_base = true OR application_id IS NULL

    UNION ALL

    -- Application-specific components: visible if user has a role in this app
    SELECT c.id, c.slug, c.category, c.primitive_type, c.schema, c.description
    FROM flint_a2ui.components c
    WHERE c.application_id = p_application_id
      AND c.application_id IN (
          SELECT ra.application_id
          FROM flint_a2ui.role_assignments ra
          WHERE ra.user_id = p_jwt_claims->'flint'->>'user_id'
      )
    ORDER BY category, slug;
$$;
```

---

## JWT claims template

The `applications.jwt_claims_template` JSONB maps flint-gate claim fields to A2UI role resolution keys. Example:

```json
{
    "user_id_claim": "flint.user_id",
    "organization_claim": "flint.organization_id",
    "roles_claim": "flint.roles"
}
```

This is used by `resolve_components()` to extract the correct `user_id` from whatever JWT shape flint-gate mints.
