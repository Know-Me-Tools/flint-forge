-- Migration: 0009_flint_a2ui_design_systems.sql
-- Adds design system import support (p5-c013).
--
-- 1. Extends flint_a2ui.design_systems with import provenance columns.
-- 2. Creates flint_a2ui.component_overrides for per-design-system prop/CSS overrides.

-- ── Extend design_systems ────────────────────────────────────────────────────

-- source_format: 'design_md' | 'w3c_tokens' | 'figma_tokens' | 'manual'
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'flint_a2ui' AND table_name = 'design_systems'
          AND column_name = 'source_format'
    ) THEN
        ALTER TABLE flint_a2ui.design_systems
            ADD COLUMN source_format text DEFAULT 'manual'
                CHECK (source_format IN ('design_md', 'w3c_tokens', 'figma_tokens', 'manual'));
    END IF;
END $$;

-- source_content: raw imported content (DESIGN.md text or token JSON)
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'flint_a2ui' AND table_name = 'design_systems'
          AND column_name = 'source_content'
    ) THEN
        ALTER TABLE flint_a2ui.design_systems ADD COLUMN source_content text;
    END IF;
END $$;

-- imported_at: timestamp of last successful import
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'flint_a2ui' AND table_name = 'design_systems'
          AND column_name = 'imported_at'
    ) THEN
        ALTER TABLE flint_a2ui.design_systems ADD COLUMN imported_at timestamptz;
    END IF;
END $$;

-- ── component_overrides ──────────────────────────────────────────────────────
-- Per-design-system component customizations. Rows here override the base
-- component definition for a specific application/design system context.

CREATE TABLE IF NOT EXISTS flint_a2ui.component_overrides (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    design_system_id uuid NOT NULL
        REFERENCES flint_a2ui.design_systems(id) ON DELETE CASCADE,
    component_id    uuid NOT NULL
        REFERENCES flint_a2ui.components(id) ON DELETE CASCADE,
    -- Prop defaults to merge over the base component schema defaults.
    prop_defaults   jsonb NOT NULL DEFAULT '{}',
    -- CSS custom property overrides, e.g. {"--btn-primary-bg": "#1d4ed8"}.
    css_vars        jsonb NOT NULL DEFAULT '{}',
    -- Override which React component to import (null = use SDK default).
    react_component text,
    -- Override which Flutter widget to instantiate (null = use SDK default).
    flutter_widget  text,
    -- Override the Askama template path for HTMX rendering (null = default).
    htmx_template   text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE (design_system_id, component_id)
);

-- RLS: only authenticated users can read; only service role can write.
ALTER TABLE flint_a2ui.component_overrides ENABLE ROW LEVEL SECURITY;

CREATE POLICY component_overrides_read ON flint_a2ui.component_overrides
    FOR SELECT
    USING (true);

CREATE POLICY component_overrides_write ON flint_a2ui.component_overrides
    FOR ALL
    USING (current_setting('role') = 'service_role');

-- Index for the common "get all overrides for a design system" query.
CREATE INDEX IF NOT EXISTS component_overrides_ds_idx
    ON flint_a2ui.component_overrides (design_system_id);
