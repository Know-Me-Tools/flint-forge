# p5-c014 — SDK Component Override + Design Token Schema Extensions

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P1 (required before p5-c010, p5-c011, p5-c013 can be completed)  
**Depends on:** p5-c001 (flint_a2ui schema), p5-c005 (application model)  
**Blocks:** p5-c010 (React SDK), p5-c011 (Flutter SDK), p5-c013 (OpenDesign integration)

---

## What this change delivers

Database schema additions and Rust type extensions that support:
1. **Per-application, per-design-system component overrides** — prop defaults, CSS class overrides, component replacement mappings
2. **SDK-specific renderer metadata** — which renderers (react, flutter, htmx) implement each component
3. **Design token format** — structured JSONB schema for the `design_systems.tokens` column
4. **Component override resolution** — extended `resolve_components()` function that merges base + app + design-system overrides

---

## Schema Changes

### 1. `flint_a2ui.components` — Add SDK Renderer Metadata

```sql
ALTER TABLE flint_a2ui.components
  ADD COLUMN renderers jsonb NOT NULL DEFAULT '{"react": true, "flutter": true, "htmx": true}'::jsonb,
  ADD COLUMN react_pkg  text,          -- npm package name if custom (default: @flint/react)
  ADD COLUMN flutter_pkg text,         -- pub package name if custom (default: flint_genui)
  ADD COLUMN htmx_template text;       -- Askama template path override

-- renderers example:
-- {"react": true, "flutter": false, "htmx": true}
-- Components marked flutter: false won't be included in FlintCatalog
```

### 2. `flint_a2ui.component_overrides` (new table)

```sql
CREATE TABLE flint_a2ui.component_overrides (
  id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  component_id     uuid NOT NULL REFERENCES flint_a2ui.components(id) ON DELETE CASCADE,
  application_id   uuid REFERENCES flint_a2ui.applications(id) ON DELETE CASCADE,
  design_system_id uuid REFERENCES flint_a2ui.design_systems(id) ON DELETE SET NULL,
  
  -- What is being overridden
  prop_defaults    jsonb,              -- Default prop values for this component in this app
  css_class_map    jsonb,              -- {"data-flint-component": "my-custom-class"}
  css_vars         jsonb,              -- {"--flint-color-primary": "#0066ff"}
  react_component  text,              -- Override React component (fully qualified import path)
  flutter_widget   text,              -- Override Flutter widget class name
  htmx_template    text,              -- Override Askama template path
  
  -- Metadata
  source           text CHECK (source IN ('manual', 'design-md', 'w3c-tokens', 'claude-design')),
  created_at       timestamptz NOT NULL DEFAULT now(),
  updated_at       timestamptz NOT NULL DEFAULT now(),
  
  -- One override per component per app per design-system
  UNIQUE (component_id, application_id, design_system_id)
);

-- RLS: application-scoped (app admin can manage their own overrides)
ALTER TABLE flint_a2ui.component_overrides ENABLE ROW LEVEL SECURITY;

CREATE POLICY "app_admin_manage_overrides" ON flint_a2ui.component_overrides
  USING (
    application_id IS NULL  -- base overrides (no app scope)
    OR application_id IN (
      SELECT app_id FROM flint_a2ui.role_assignments
      WHERE user_id = (current_setting('app.jwt_claims', true)::jsonb->>'flint.user_id')::uuid
        AND role_name = 'app-admin'
    )
  );

COMMENT ON TABLE flint_a2ui.component_overrides IS 
  'Per-application, per-design-system overrides for component props, CSS, and renderer mapping';
```

### 3. `flint_a2ui.design_systems` — Add Import Metadata Columns

```sql
ALTER TABLE flint_a2ui.design_systems
  ADD COLUMN source_format  text CHECK (source_format IN ('manual', 'design-md', 'w3c-tokens', 'claude-design', 'odsf')),
  ADD COLUMN source_content text,       -- Raw imported content for re-parsing
  ADD COLUMN imported_at    timestamptz,
  ADD COLUMN token_schema_version text DEFAULT 'w3c-2024'; -- Which token schema the tokens column conforms to
```

### 4. Design Token JSONB Schema (W3C Design Tokens Community Group 2024)

The `design_systems.tokens` column must store W3C-compatible design tokens:

```json
{
  "$schema": "https://design-tokens.org/schema/2024",
  "color": {
    "primary":   { "$value": "oklch(68% 0.21 250)", "$type": "color" },
    "surface":   { "$value": "oklch(98% 0 0)", "$type": "color" },
    "text":      { "$value": "oklch(18% 0 0)", "$type": "color" },
    "accent":    { "$value": "{color.primary}", "$type": "color" }
  },
  "typography": {
    "fontSans":  { "$value": "Inter, system-ui, sans-serif", "$type": "fontFamily" },
    "textBase":  { "$value": "clamp(1rem, 0.92rem + 0.4vw, 1.125rem)", "$type": "dimension" },
    "textHero":  { "$value": "clamp(3rem, 1rem + 7vw, 8rem)", "$type": "dimension" }
  },
  "spacing": {
    "sm":        { "$value": "0.5rem", "$type": "dimension" },
    "md":        { "$value": "1rem", "$type": "dimension" },
    "lg":        { "$value": "2rem", "$type": "dimension" },
    "section":   { "$value": "clamp(4rem, 3rem + 5vw, 10rem)", "$type": "dimension" }
  },
  "motion": {
    "durationFast":   { "$value": "150ms", "$type": "duration" },
    "durationNormal": { "$value": "300ms", "$type": "duration" },
    "easeOutExpo":    { "$value": "cubic-bezier(0.16, 1, 0.3, 1)", "$type": "cubicBezier" }
  },
  "radius": {
    "sm":  { "$value": "0.375rem", "$type": "dimension" },
    "md":  { "$value": "0.5rem", "$type": "dimension" },
    "lg":  { "$value": "0.75rem", "$type": "dimension" }
  }
}
```

### 5. Extended `resolve_components()` Function

```sql
CREATE OR REPLACE FUNCTION flint_a2ui.resolve_components_with_overrides(
  p_application_id uuid,
  p_jwt_claims jsonb,
  p_design_system_id uuid DEFAULT NULL
) RETURNS TABLE (
  slug           text,
  primitive_type text,
  category       text,
  schema         jsonb,
  description    text,
  renderers      jsonb,
  prop_defaults  jsonb,        -- merged from component_overrides
  css_vars       jsonb,        -- merged from component_overrides + design_systems.tokens
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
    COALESCE(co.prop_defaults, '{}'::jsonb)       AS prop_defaults,
    COALESCE(co.css_vars, '{}'::jsonb)            AS css_vars,
    COALESCE(co.react_component, c.react_pkg)     AS react_component,
    COALESCE(co.flutter_widget, c.flutter_pkg)    AS flutter_widget,
    COALESCE(co.htmx_template, c.htmx_template)  AS htmx_template
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
      WHERE ra.user_id = (p_jwt_claims->>'flint.user_id')::uuid
        AND ra.app_id = p_application_id
    )
  ORDER BY c.category, c.slug;
$$;
```

---

## Rust Type Additions

```rust
// crates/fdb-app/src/a2ui/types.rs (additions)

/// Component definition returned by resolve_components_with_overrides()
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedComponent {
    pub slug: String,
    pub primitive_type: String,
    pub category: String,
    pub schema: serde_json::Value,
    pub description: Option<String>,
    pub renderers: Renderers,
    pub prop_defaults: serde_json::Value,
    pub css_vars: serde_json::Value,
    pub react_component: Option<String>,
    pub flutter_widget: Option<String>,
    pub htmx_template: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Renderers {
    pub react: bool,
    pub flutter: bool,
    pub htmx: bool,
}

/// W3C Design Token value (2024 format)
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DesignToken {
    #[serde(rename = "$value")]
    pub value: String,
    #[serde(rename = "$type")]
    pub token_type: String,
}
```

---

## Migration File

`migrations/0003_flint_a2ui_sdk_extensions.sql`

---

## Gate Tests

- [ ] `component_overrides` table created with correct RLS
- [ ] `resolve_components_with_overrides()` returns base components for null application_id
- [ ] Override row applied: `prop_defaults` merged correctly when override exists
- [ ] `renderers` column updated: `{"react": true, "flutter": false, "htmx": true}` filters flutter from catalog
- [ ] W3C token schema stored and retrievable from `design_systems.tokens`
- [ ] `source_format = 'design-md'` imports round-trip through `source_content` column
