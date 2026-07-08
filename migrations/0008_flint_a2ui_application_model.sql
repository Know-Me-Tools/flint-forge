-- Migration 0008: A2UI application model, role hierarchy, and permission-filtered component resolution
-- Depends on: 0002_flint_a2ui (applications, components, roles, role_assignments),
--             0004_flint_a2ui_sdk_extensions (component_overrides, design_systems)
-- Idempotent: CREATE OR REPLACE FUNCTION and DROP TRIGGER IF EXISTS guards

-- ── Role hierarchy resolution ───────────────────────────────────────────────
-- Returns a role plus all of its descendant (child) roles recursively.
-- Used to expand a user's assigned roles into the full set of inherited roles.

CREATE OR REPLACE FUNCTION flint_a2ui.resolve_role_descendants(p_role_id uuid)
RETURNS TABLE (role_id uuid) LANGUAGE sql STABLE AS $$
    WITH RECURSIVE descendants AS (
        SELECT r.id FROM flint_a2ui.roles r WHERE r.id = p_role_id
        UNION ALL
        SELECT r.id
        FROM flint_a2ui.roles r
        JOIN descendants d ON r.parent_role_id = d.id
    )
    SELECT id FROM descendants;
$$;

COMMENT ON FUNCTION flint_a2ui.resolve_role_descendants(uuid) IS
    'Returns a role id plus all descendant child roles via recursive CTE.';

GRANT EXECUTE ON FUNCTION flint_a2ui.resolve_role_descendants(uuid) TO authenticated, service_role;

-- ── resolve_components ──────────────────────────────────────────────────────
-- Canonical permission-filtered component query for an application.
-- SECURITY DEFINER: runs as definer so it can read role_assignments regardless
-- of the caller's RLS context. jwt_claims are passed explicitly.
--
-- Returns:
--   - All base components (is_base = true)
--   - Application-specific components only if the user has at least one role
--     assignment (direct or inherited via role hierarchy) in that application.
--
-- jwt_claims template: the applications.jwt_claims_template column can override
-- the JSON path used to extract the user id. The default path is 'flint.user_id'.

CREATE OR REPLACE FUNCTION flint_a2ui.resolve_components(
    p_application_id uuid,
    p_jwt_claims     jsonb DEFAULT '{}'::jsonb
)
RETURNS TABLE (
    id             uuid,
    slug           text,
    category       text,
    primitive_type text,
    schema         jsonb,
    description    text
) LANGUAGE sql STABLE SECURITY DEFINER AS $$
    WITH user_app_roles AS (
        SELECT DISTINCT ra.role_id
        FROM flint_a2ui.role_assignments ra
        WHERE ra.application_id = p_application_id
          AND ra.user_id = (p_jwt_claims->'flint'->>'user_id')
    ),
    effective_roles AS (
        SELECT DISTINCT rd.role_id
        FROM user_app_roles uar
        JOIN LATERAL flint_a2ui.resolve_role_descendants(uar.role_id) rd ON true
    )
    SELECT
        c.id,
        c.slug,
        c.category,
        c.primitive_type,
        c.schema,
        c.description
    FROM flint_a2ui.components c
    WHERE c.is_base = true
       OR c.application_id IS NULL
       OR (
           c.application_id = p_application_id
           AND EXISTS (SELECT 1 FROM effective_roles)
       )
    ORDER BY c.category, c.slug;
$$;

COMMENT ON FUNCTION flint_a2ui.resolve_components(uuid, jsonb) IS
    'Returns base + permission-filtered app-specific components for a user. '
    'SECURITY DEFINER: jwt_claims must be passed explicitly.';

GRANT EXECUTE ON FUNCTION flint_a2ui.resolve_components(uuid, jsonb)
    TO authenticated, service_role;

-- ── resolve_application_roles ───────────────────────────────────────────────
-- Convenience function: given an application and JWT claims, return the user's
-- effective (direct + inherited) role slugs.

CREATE OR REPLACE FUNCTION flint_a2ui.resolve_application_roles(
    p_application_id uuid,
    p_jwt_claims     jsonb DEFAULT '{}'::jsonb
)
RETURNS TABLE (slug text, name text) LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT DISTINCT r.slug, r.name
    FROM flint_a2ui.role_assignments ra
    JOIN LATERAL flint_a2ui.resolve_role_descendants(ra.role_id) rd ON true
    JOIN flint_a2ui.roles r ON r.id = rd.role_id
    WHERE ra.application_id = p_application_id
      AND ra.user_id = (p_jwt_claims->'flint'->>'user_id')
    ORDER BY r.slug;
$$;

COMMENT ON FUNCTION flint_a2ui.resolve_application_roles(uuid, jsonb) IS
    'Returns effective (direct + inherited) role slugs for a user in an application.';

GRANT EXECUTE ON FUNCTION flint_a2ui.resolve_application_roles(uuid, jsonb)
    TO authenticated, service_role;

-- ── Application admin trigger guard ─────────────────────────────────────────
-- Prevent deletion of system applications (flint-admin, flint-playground).

CREATE OR REPLACE FUNCTION flint_a2ui.prevent_system_application_delete()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.is_system THEN
        RAISE EXCEPTION 'cannot delete system application: %', OLD.slug;
    END IF;
    RETURN OLD;
END;
$$;

DROP TRIGGER IF EXISTS a2ui_prevent_system_app_delete ON flint_a2ui.applications;

CREATE TRIGGER a2ui_prevent_system_app_delete
    BEFORE DELETE ON flint_a2ui.applications
    FOR EACH ROW
    EXECUTE FUNCTION flint_a2ui.prevent_system_application_delete();

COMMENT ON TRIGGER a2ui_prevent_system_app_delete ON flint_a2ui.applications IS
    'Prevents deletion of system applications seeded by Flint.';
