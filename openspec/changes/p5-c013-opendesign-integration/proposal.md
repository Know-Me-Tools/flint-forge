# p5-c013 — OpenDesign + Claude Design Integration

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P2 (enables design-system-driven component overrides and ideation workflows)  
**Depends on:** p5-c010 (React SDK — DESIGN.md → CSS token injection), p5-c012 (HTMX renderer — HTML preview fragments)  
**Blocks:** p5-c015 (Claude Design skill package)

---

## What this change delivers

Integration with both **OpenDesign** (`nexu-io/open-design`, 73k stars, Apache-2.0) and **Claude Design** (Anthropic, launched April 2026) so that Flint component libraries can be used within design exploration workflows and so design systems produced in those tools can feed back into the `flint_a2ui.design_systems` table.

---

## Research Basis

See `.firecrawl/opendesign-claude-design-2026.md` for full findings. Key facts:

- **OpenDesign** (repo: `github.com/nexu-io/open-design`): Uses **DESIGN.md** 9-section format, **SKILL.md** plugins with `od:` frontmatter, REST API `/api/plugins`, MCP server (`od mcp`). Drop a folder into `skills/` → component appears in picker. Drop a `DESIGN.md` into `design-systems/<brand>/` → design system active.
- **Claude Design**: Anthropic product with `/design-sync` (imports codebase design system) and `/design` command for creation. Claude Design exports can be imported into OpenDesign via ZIP drop.
- Both use **SKILL.md** as the skill format; Claude Code recognizes it via `claude plugin marketplace add`.

---

## Two Integration Surfaces

### Surface A: Flint → OpenDesign (Outbound)

Expose Flint components as an OpenDesign plugin so that ideation/design workflows can use Flint components directly.

**Deliverable**: An OpenDesign plugin package at `plugins/flint-components/` (or distributed via GitHub).

**Plugin structure** (OpenDesign `open-design.json` format):
```json
{
  "name": "flint-components",
  "version": "0.1.0",
  "description": "Flint A2UI component catalog for OpenDesign",
  "inputs": [
    { "name": "endpoint", "type": "string", "label": "Flint API endpoint" },
    { "name": "applicationId", "type": "string", "label": "Application ID" }
  ],
  "capabilities": ["component-catalog", "design-system-import"],
  "skills": [
    "skills/flint-component-browser/SKILL.md",
    "skills/flint-surface-preview/SKILL.md"
  ]
}
```

**Skills provided**:
- `flint-component-browser`: Queries `GET /a2ui/v1/components` and displays in OpenDesign picker
- `flint-surface-preview`: Calls `POST /htmx/components/:slug` → renders HTMX HTML preview inline

### Surface B: OpenDesign / Claude Design → Flint (Inbound)

Import a DESIGN.md or Claude Design export into `flint_a2ui.design_systems`.

**New REST endpoint** in `fdb-gateway`:
```
POST /a2ui/v1/design-systems/import
Content-Type: application/json

{
  "format": "design-md" | "w3c-tokens" | "claude-design-zip",
  "content": "...",        // DESIGN.md text OR W3C token JSON
  "application_id": "uuid" // optional scope
}
```

**DESIGN.md parser** (Rust, in `fdb-app/src/a2ui/design_md_parser.rs`):
```rust
pub struct DesignMd {
    pub color: HashMap<String, String>,
    pub typography: TypographyConfig,
    pub spacing: SpacingScale,
    pub layout: LayoutConfig,
    pub components: Vec<ComponentOverrideSpec>,
    pub motion: MotionConfig,
    pub voice: VoiceConfig,
    pub brand: BrandConfig,
    pub anti_patterns: Vec<String>,
}

pub fn parse_design_md(content: &str) -> Result<DesignMd, DesignMdError>;
pub fn design_md_to_tokens(md: &DesignMd) -> serde_json::Value;
pub fn design_md_to_component_overrides(md: &DesignMd) -> Vec<ComponentOverride>;
```

**W3C Design Token format** (standard JSON, W3C Design Tokens Community Group):
```json
{
  "color": {
    "primary": { "$value": "#6366f1", "$type": "color" },
    "surface": { "$value": "{color.neutral.100}", "$type": "color" }
  },
  "spacing": {
    "sm": { "$value": "0.5rem", "$type": "dimension" }
  }
}
```
This is stored directly in `flint_a2ui.design_systems.tokens jsonb`.

**Claude Design ZIP import**: Parse the ZIP exported from `claude.ai` design projects. Extract DESIGN.md and any token files, feed through the DESIGN.md parser.

---

## Database Schema Additions (requires p5-c014)

```sql
-- Extension to flint_a2ui.design_systems
ALTER TABLE flint_a2ui.design_systems 
  ADD COLUMN source_format text CHECK (source_format IN ('manual', 'design-md', 'w3c-tokens', 'claude-design', 'odsf')),
  ADD COLUMN source_content text,         -- Raw imported content (DESIGN.md text, etc.)
  ADD COLUMN imported_at timestamptz;

-- New table: component overrides per design system
CREATE TABLE flint_a2ui.component_overrides (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  component_id uuid NOT NULL REFERENCES flint_a2ui.components(id),
  design_system_id uuid NOT NULL REFERENCES flint_a2ui.design_systems(id),
  application_id uuid REFERENCES flint_a2ui.applications(id),
  override_schema jsonb NOT NULL,         -- Prop value overrides (CSS token mappings)
  css_overrides  jsonb,                   -- CSS custom property value map
  notes          text,
  created_at     timestamptz NOT NULL DEFAULT now(),
  UNIQUE (component_id, design_system_id, application_id)
);

COMMENT ON TABLE flint_a2ui.component_overrides IS 
  'Per-design-system/application overrides for component appearance. Populated from DESIGN.md imports.';
```

---

## Claude Design via `/design-sync`

When a user runs `/design-sync` in Claude Code pointing at a Flint project:
1. Claude Code reads `packages/flint-react/src/tokens/FlintTokens.ts` (or CSS custom properties)
2. Imports the token set into Claude Design
3. User can visually design component layouts in Claude Design
4. Export ZIP → drop into OpenDesign or `POST /a2ui/v1/design-systems/import`

Flint's React SDK exports tokens in a format compatible with this flow via:
```ts
// packages/flint-react/src/tokens/export.ts
export function exportDesignSyncTokens(): Record<string, string> {
  // Returns CSS custom property map in Claude Design compatible format
}
```

---

## Gate Tests

- [ ] `POST /a2ui/v1/design-systems/import` with a DESIGN.md file creates a `flint_a2ui.design_systems` row
- [ ] W3C design token JSON import creates correct `tokens jsonb` column value
- [ ] `component_overrides` table created, RLS applied (application-scoped)
- [ ] HTMX component preview fragment reflects imported design tokens (CSS vars injected)
- [ ] OpenDesign plugin package structure validates against `open-design.json` schema
- [ ] `flint-component-browser` SKILL.md queries Flint REST API and returns component list
