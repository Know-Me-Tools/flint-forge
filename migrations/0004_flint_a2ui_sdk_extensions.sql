-- Migration 0004: A2UI SDK component override and design token schema extensions
-- Depends on: 0002_flint_a2ui (flint_a2ui schema), 0003_a2ui_triggers
-- Adds: renderer metadata, component_overrides table, design_systems import columns,
--       resolve_components_with_overrides() function

-- ── Add renderer metadata to flint_a2ui.components ──────────────────────────

ALTER TABLE flint_a2ui.components
    ADD COLUMN IF NOT EXISTS renderers     jsonb NOT NULL DEFAULT '{"react": true, "flutter": true, "htmx": true}'::jsonb,
    ADD COLUMN IF NOT EXISTS react_pkg     text,
    ADD COLUMN IF NOT EXISTS flutter_pkg   text,
    ADD COLUMN IF NOT EXISTS htmx_template text;

COMMENT ON COLUMN flint_a2ui.components.renderers IS
    'Which SDK renderers support this component. e.g. {"react": true, "flutter": false, "htmx": true}';
COMMENT ON COLUMN flint_a2ui.components.react_pkg IS
    'npm package name for custom React renderer (default: @flint/react)';
COMMENT ON COLUMN flint_a2ui.components.flutter_pkg IS
    'pub.dev package name for custom Flutter renderer (default: flint_genui)';
COMMENT ON COLUMN flint_a2ui.components.htmx_template IS
    'Askama template path override for HTMX renderer';

-- ── Per-application, per-design-system component overrides ──────────────────

CREATE TABLE IF NOT EXISTS flint_a2ui.component_overrides (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    component_id     uuid NOT NULL REFERENCES flint_a2ui.components(id) ON DELETE CASCADE,
    application_id   uuid REFERENCES flint_a2ui.applications(id) ON DELETE CASCADE,
    design_system_id uuid REFERENCES flint_a2ui.design_systems(id) ON DELETE SET NULL,
    prop_defaults    jsonb,
    css_class_map    jsonb,
    css_vars         jsonb,
    react_component  text,
    flutter_widget   text,
    htmx_template    text,
    source           text CHECK (source IN ('manual', 'design-md', 'w3c-tokens', 'claude-design')),
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    UNIQUE (component_id, application_id, design_system_id)
);

COMMENT ON TABLE flint_a2ui.component_overrides IS
    'Per-application, per-design-system overrides for component props, CSS, and renderer mapping.';

-- RLS: app-admin may manage overrides for their own application.
-- SECURITY: uses application_id column from role_assignments (not app_id — R2 fix).
ALTER TABLE flint_a2ui.component_overrides ENABLE ROW LEVEL SECURITY;

CREATE POLICY "app_admin_manage_overrides" ON flint_a2ui.component_overrides
    USING (
        application_id IS NULL
        OR application_id IN (
            SELECT ra.application_id
            FROM flint_a2ui.role_assignments ra
            JOIN flint_a2ui.roles r ON r.id = ra.role_id
            WHERE ra.user_id = (current_setting('app.jwt_claims', true)::jsonb->>'flint.user_id')
              AND r.slug = 'app-admin'
        )
    );

GRANT SELECT ON flint_a2ui.component_overrides TO authenticated;
GRANT INSERT, UPDATE, DELETE ON flint_a2ui.component_overrides TO authenticated;
GRANT SELECT, INSERT, UPDATE, DELETE ON flint_a2ui.component_overrides TO service_role;

-- ── Design system import metadata ───────────────────────────────────────────

ALTER TABLE flint_a2ui.design_systems
    ADD COLUMN IF NOT EXISTS source_format        text CHECK (source_format IN ('manual', 'design-md', 'w3c-tokens', 'claude-design', 'odsf')),
    ADD COLUMN IF NOT EXISTS source_content       text,
    ADD COLUMN IF NOT EXISTS imported_at          timestamptz,
    ADD COLUMN IF NOT EXISTS token_schema_version text NOT NULL DEFAULT 'w3c-2024';

COMMENT ON COLUMN flint_a2ui.design_systems.source_format IS
    'Format of the source that produced this design system (design-md, w3c-tokens, etc.)';
COMMENT ON COLUMN flint_a2ui.design_systems.source_content IS
    'Raw imported content for re-parsing and round-trip import';
COMMENT ON COLUMN flint_a2ui.design_systems.token_schema_version IS
    'W3C Design Tokens Community Group schema version used by the tokens column';

-- ── resolve_components_with_overrides() ─────────────────────────────────────
-- Returns components merged with app-scoped and design-system-scoped overrides.
-- SECURITY DEFINER: runs as definer so it can read role_assignments regardless of
-- the caller's role. jwt_claims are passed explicitly — never read from GUC here.

CREATE OR REPLACE FUNCTION flint_a2ui.resolve_components_with_overrides(
    p_application_id  uuid,
    p_jwt_claims      jsonb,
    p_design_system_id uuid DEFAULT NULL
)
RETURNS TABLE (
    slug            text,
    primitive_type  text,
    category        text,
    schema          jsonb,
    description     text,
    renderers       jsonb,
    prop_defaults   jsonb,
    css_vars        jsonb,
    react_component text,
    flutter_widget  text,
    htmx_template   text
) LANGUAGE sql SECURITY DEFINER AS $$
    SELECT
        c.slug,
        c.primitive_type,
        c.category,
        c.schema,
        c.description,
        c.renderers,
        COALESCE(co.prop_defaults, '{}'::jsonb)            AS prop_defaults,
        COALESCE(co.css_vars, '{}'::jsonb)                 AS css_vars,
        COALESCE(co.react_component, c.react_pkg)          AS react_component,
        COALESCE(co.flutter_widget, c.flutter_pkg)         AS flutter_widget,
        COALESCE(co.htmx_template, c.htmx_template)       AS htmx_template
    FROM flint_a2ui.components c
    LEFT JOIN flint_a2ui.component_overrides co
        ON co.component_id = c.id
       AND (co.application_id IS NULL OR co.application_id = p_application_id)
       AND (co.design_system_id IS NULL OR co.design_system_id = p_design_system_id)
    WHERE
        c.is_base = true
        OR c.application_id = p_application_id
        OR EXISTS (
            SELECT 1 FROM flint_a2ui.role_assignments ra
            WHERE ra.user_id = (p_jwt_claims->>'flint.user_id')
              AND ra.application_id = p_application_id
        )
    ORDER BY c.category, c.slug;
$$;

COMMENT ON FUNCTION flint_a2ui.resolve_components_with_overrides(uuid, jsonb, uuid) IS
    'Returns components for an application, merging base + app + design-system overrides. '
    'SECURITY DEFINER: jwt_claims must be passed explicitly, never read from GUC.';

GRANT EXECUTE ON FUNCTION flint_a2ui.resolve_components_with_overrides(uuid, jsonb, uuid)
    TO authenticated, service_role;
