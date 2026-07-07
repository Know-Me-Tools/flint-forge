//! DESIGN.md parser — 9-section Flint design specification format.
//!
//! DESIGN.md is a structured Markdown file that captures a project's complete
//! design system. It is produced by Claude Design (via `/design-sync`) and can
//! also be authored manually. This parser extracts the 9 sections and maps
//! them to `DesignTokenMap` + `ComponentOverride` structs consumable by the
//! A2UI import pipeline.
//!
//! # Section structure
//!
//! ```markdown
//! # Design System — <Name>
//!
//! ## 1. Color
//! ## 2. Typography
//! ## 3. Spacing
//! ## 4. Layout
//! ## 5. Components
//! ## 6. Motion
//! ## 7. Voice
//! ## 8. Brand
//! ## 9. Anti-patterns
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A parsed DESIGN.md document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DesignMd {
    /// Human-readable name extracted from the H1 heading.
    pub name: String,
    /// Raw tokens extracted from §1–§4 (color, typography, spacing, layout).
    pub tokens: serde_json::Value,
    /// Component-level overrides from §5.
    pub component_overrides: Vec<ComponentOverride>,
    /// Motion / animation configuration from §6.
    pub motion: serde_json::Value,
    /// Voice / tone guidelines from §7 (stored as raw text).
    pub voice: String,
    /// Brand summary from §8 (stored as raw text).
    pub brand: String,
    /// Anti-patterns list from §9 (stored as raw text).
    pub anti_patterns: String,
    /// Raw section text by section number (1–9) for full fidelity.
    pub raw_sections: HashMap<u8, String>,
}

/// A single component override extracted from §5 of DESIGN.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOverride {
    /// Matches `flint_a2ui.components.slug`.
    pub slug: String,
    /// Prop defaults to merge over base component defaults.
    pub prop_defaults: serde_json::Value,
    /// CSS custom property overrides.
    pub css_vars: serde_json::Value,
    /// Optional renderer overrides.
    pub react_component: Option<String>,
    pub flutter_widget: Option<String>,
    pub htmx_template: Option<String>,
}

/// Errors from DESIGN.md parsing.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("missing H1 title")]
    MissingTitle,
    #[error("invalid JSON in section {section}: {source}")]
    InvalidJson {
        section: u8,
        source: serde_json::Error,
    },
}

/// Parse a DESIGN.md string into a `DesignMd` document.
pub fn parse(input: &str) -> Result<DesignMd, ParseError> {
    let name = extract_title(input).ok_or(ParseError::MissingTitle)?;
    let sections = split_sections(input);
    let mut tokens = serde_json::Value::Object(serde_json::Map::new());
    let mut motion = serde_json::Value::Object(serde_json::Map::new());
    let mut component_overrides = Vec::new();

    // §1 Color
    if let Some(text) = sections.get(&1) {
        let color = extract_json_blocks(text);
        if let Some(color_json) = color.first() {
            tokens["color"] = serde_json::from_str(color_json)
                .map_err(|e| ParseError::InvalidJson { section: 1, source: e })?;
        } else {
            tokens["color"] = extract_kv_as_object(text);
        }
    }

    // §2 Typography
    if let Some(text) = sections.get(&2) {
        let json_blocks = extract_json_blocks(text);
        if let Some(j) = json_blocks.first() {
            tokens["typography"] = serde_json::from_str(j)
                .map_err(|e| ParseError::InvalidJson { section: 2, source: e })?;
        } else {
            tokens["typography"] = extract_kv_as_object(text);
        }
    }

    // §3 Spacing
    if let Some(text) = sections.get(&3) {
        let json_blocks = extract_json_blocks(text);
        if let Some(j) = json_blocks.first() {
            tokens["spacing"] = serde_json::from_str(j)
                .map_err(|e| ParseError::InvalidJson { section: 3, source: e })?;
        } else {
            tokens["spacing"] = extract_kv_as_object(text);
        }
    }

    // §4 Layout
    if let Some(text) = sections.get(&4) {
        tokens["layout"] = extract_kv_as_object(text);
    }

    // §5 Components
    if let Some(text) = sections.get(&5) {
        component_overrides = parse_component_overrides(text);
    }

    // §6 Motion
    if let Some(text) = sections.get(&6) {
        motion = extract_kv_as_object(text);
        tokens["motion"] = motion.clone();
    }

    // §7–§9 stored as raw text
    let voice = sections.get(&7).cloned().unwrap_or_default();
    let brand = sections.get(&8).cloned().unwrap_or_default();
    let anti_patterns = sections.get(&9).cloned().unwrap_or_default();

    Ok(DesignMd {
        name,
        tokens,
        component_overrides,
        motion,
        voice,
        brand,
        anti_patterns,
        raw_sections: sections,
    })
}

// ─── W3C Design Token mapper ─────────────────────────────────────────────────

/// Convert W3C Design Token Community Group 2024 JSON into Flint's `tokens`
/// jsonb shape. Flattens nested token groups into a two-level structure
/// `{ "color": { "primary": "#..." }, "spacing": { "md": "16px" }, ... }`.
pub fn map_w3c_tokens(input: &str) -> Result<serde_json::Value, serde_json::Error> {
    let raw: serde_json::Value = serde_json::from_str(input)?;
    let mut result = serde_json::Map::new();
    if let Some(obj) = raw.as_object() {
        for (category, value) in obj {
            if category.starts_with('$') {
                continue; // skip $schema, $description etc.
            }
            result.insert(
                category.clone(),
                flatten_w3c_group(value),
            );
        }
    }
    Ok(serde_json::Value::Object(result))
}

/// Flatten a W3C token group to a simple { name: value } map.
fn flatten_w3c_group(group: &serde_json::Value) -> serde_json::Value {
    let mut flat = serde_json::Map::new();
    if let Some(obj) = group.as_object() {
        for (key, val) in obj {
            if key.starts_with('$') {
                continue;
            }
            if let Some(token_value) = val.get("$value") {
                flat.insert(key.clone(), token_value.clone());
            } else if val.is_object() {
                // Nested group — recurse and merge with key prefix
                if let Some(nested) = flatten_w3c_group(val).as_object() {
                    for (nk, nv) in nested {
                        flat.insert(format!("{key}-{nk}"), nv.clone());
                    }
                }
            }
        }
    }
    serde_json::Value::Object(flat)
}

// ─── Internal helpers ────────────────────────────────────────────────────────

/// Extract the H1 title from the document.
fn extract_title(input: &str) -> Option<String> {
    for line in input.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            // Strip "Design System — " prefix if present
            let name = rest
                .trim_start_matches("Design System — ")
                .trim_start_matches("Design System: ")
                .trim()
                .to_owned();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Split the document into numbered sections by H2 headings matching
/// `## <N>.` or `## <N> `. Returns a map of section number → section body text.
fn split_sections(input: &str) -> HashMap<u8, String> {
    let mut sections: HashMap<u8, String> = HashMap::new();
    let mut current_num: Option<u8> = None;
    let mut current_body = String::new();

    for line in input.lines() {
        let trimmed = line.trim();
        // Match "## 1. Color" or "## 1 Color"
        if let Some(rest) = trimmed.strip_prefix("## ") {
            if let Some(num) = parse_section_number(rest) {
                if let Some(n) = current_num {
                    sections.insert(n, current_body.trim().to_owned());
                }
                current_num = Some(num);
                current_body = String::new();
                continue;
            }
        }
        if current_num.is_some() {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if let Some(n) = current_num {
        sections.insert(n, current_body.trim().to_owned());
    }
    sections
}

fn parse_section_number(heading: &str) -> Option<u8> {
    let digits: String = heading.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u8>().ok().filter(|&n| (1..=9).contains(&n))
}

/// Extract fenced JSON code blocks from a section.
fn extract_json_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if !in_block && (trimmed.starts_with("```json") || trimmed == "```{") {
            in_block = true;
            current.clear();
        } else if in_block && trimmed == "```" {
            blocks.push(current.trim().to_owned());
            current.clear();
            in_block = false;
        } else if in_block {
            current.push_str(line);
            current.push('\n');
        }
    }
    blocks
}

/// Parse `key: value` lines in a section into a JSON object.
/// Recognises `#rrggbb` hex colors, `Npx`/`Nrem` dimensions, bare strings.
fn extract_kv_as_object(text: &str) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with('-') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().replace(' ', "_").to_lowercase();
            let value = v.trim().trim_end_matches(',').trim_matches('"').trim().to_owned();
            if !key.is_empty() && !value.is_empty() {
                map.insert(key, serde_json::Value::String(value));
            }
        }
    }
    serde_json::Value::Object(map)
}

/// Parse §5 component override blocks. Each component starts with a heading
/// `### <slug>` and can contain JSON blocks for `prop_defaults` and `css_vars`.
fn parse_component_overrides(text: &str) -> Vec<ComponentOverride> {
    let mut overrides = Vec::new();
    let mut current_slug: Option<String> = None;
    let mut current_body = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(slug_line) = trimmed.strip_prefix("### ") {
            if let Some(slug) = current_slug.take() {
                if let Some(ov) = parse_single_override(&slug, &current_body) {
                    overrides.push(ov);
                }
            }
            current_slug = Some(slug_line.trim().to_lowercase().replace(' ', "-"));
            current_body.clear();
        } else if current_slug.is_some() {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if let Some(slug) = current_slug {
        if let Some(ov) = parse_single_override(&slug, &current_body) {
            overrides.push(ov);
        }
    }
    overrides
}

fn parse_single_override(slug: &str, body: &str) -> Option<ComponentOverride> {
    let blocks = extract_json_blocks(body);
    let prop_defaults = blocks
        .first()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    let css_vars = blocks
        .get(1)
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    // Check that at least something was found
    if prop_defaults == serde_json::json!({}) && css_vars == serde_json::json!({}) && body.trim().is_empty() {
        return None;
    }

    Some(ComponentOverride {
        slug: slug.to_owned(),
        prop_defaults,
        css_vars,
        react_component: extract_directive(body, "react_component"),
        flutter_widget: extract_directive(body, "flutter_widget"),
        htmx_template: extract_directive(body, "htmx_template"),
    })
}

fn extract_directive(body: &str, key: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}:")) {
            let value = rest.trim().trim_matches('"').to_owned();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"
# Design System — Acme Corp

## 1. Color

```json
{
  "primary": "#2563eb",
  "surface": "#ffffff",
  "text": "#0f172a"
}
```

## 2. Typography

font_family: Inter, system-ui, sans-serif
size_md: 14px
size_lg: 16px

## 3. Spacing

xs: 4px
sm: 8px
md: 16px
lg: 24px

## 4. Layout

max_width: 1280px
sidebar_width: 240px

## 5. Components

### button

```json
{ "variant": "primary", "size": "md" }
```

```json
{ "--btn-primary-bg": "#1d4ed8" }
```

react_component: "@acme/ui/Button"

### data-grid

```json
{ "pageSize": 25 }
```

## 6. Motion

duration_fast: 150ms
easing: ease-in-out

## 7. Voice

Friendly, clear, and concise.

## 8. Brand

Acme Corp: professional with warmth.

## 9. Anti-patterns

- Never use red for primary actions.
- Avoid all-caps labels.
"##;

    #[test]
    fn parses_title() {
        let doc = parse(SAMPLE).expect("parse");
        assert_eq!(doc.name, "Acme Corp");
    }

    #[test]
    fn parses_color_section() {
        let doc = parse(SAMPLE).expect("parse");
        assert_eq!(doc.tokens["color"]["primary"], "#2563eb");
    }

    #[test]
    fn parses_typography_kv() {
        let doc = parse(SAMPLE).expect("parse");
        assert_eq!(doc.tokens["typography"]["font_family"], "Inter, system-ui, sans-serif");
    }

    #[test]
    fn parses_spacing_kv() {
        let doc = parse(SAMPLE).expect("parse");
        assert_eq!(doc.tokens["spacing"]["md"], "16px");
    }

    #[test]
    fn parses_component_overrides() {
        let doc = parse(SAMPLE).expect("parse");
        assert_eq!(doc.component_overrides.len(), 2);
        let btn = doc.component_overrides.iter().find(|c| c.slug == "button").expect("button");
        assert_eq!(btn.prop_defaults["variant"], "primary");
        assert_eq!(btn.css_vars["--btn-primary-bg"], "#1d4ed8");
        assert_eq!(btn.react_component.as_deref(), Some("@acme/ui/Button"));
    }

    #[test]
    fn parses_voice_brand_anti_patterns() {
        let doc = parse(SAMPLE).expect("parse");
        assert!(doc.voice.contains("Friendly"));
        assert!(doc.brand.contains("Acme Corp"));
        assert!(doc.anti_patterns.contains("red"));
    }

    #[test]
    fn raw_sections_stored() {
        let doc = parse(SAMPLE).expect("parse");
        assert!(doc.raw_sections.contains_key(&1));
        assert!(doc.raw_sections.contains_key(&9));
    }

    #[test]
    fn w3c_token_mapper_flattens_nested() {
        let input = r##"{
          "color": {
            "$type": "color",
            "primary": { "$value": "#2563eb" },
            "brand": {
              "dark": { "$value": "#1d4ed8" }
            }
          },
          "spacing": {
            "md": { "$value": "16px" }
          }
        }"##;
        let result = map_w3c_tokens(input).expect("map");
        assert_eq!(result["color"]["primary"], "#2563eb");
        assert_eq!(result["color"]["brand-dark"], "#1d4ed8");
        assert_eq!(result["spacing"]["md"], "16px");
    }

    #[test]
    fn missing_title_returns_error() {
        let result = parse("## 1. Color\nprimary: #000");
        assert!(matches!(result, Err(ParseError::MissingTitle)));
    }
}
