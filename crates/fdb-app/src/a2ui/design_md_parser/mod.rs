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

mod components;
mod sections;
mod text_extract;
mod w3c;

#[cfg(test)]
mod tests;

pub use w3c::map_w3c_tokens;

use components::parse_component_overrides;
use sections::{extract_title, split_sections};
use text_extract::{extract_json_blocks, extract_kv_as_object};

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
            tokens["color"] =
                serde_json::from_str(color_json).map_err(|e| ParseError::InvalidJson {
                    section: 1,
                    source: e,
                })?;
        } else {
            tokens["color"] = extract_kv_as_object(text);
        }
    }

    // §2 Typography
    if let Some(text) = sections.get(&2) {
        let json_blocks = extract_json_blocks(text);
        if let Some(j) = json_blocks.first() {
            tokens["typography"] =
                serde_json::from_str(j).map_err(|e| ParseError::InvalidJson {
                    section: 2,
                    source: e,
                })?;
        } else {
            tokens["typography"] = extract_kv_as_object(text);
        }
    }

    // §3 Spacing
    if let Some(text) = sections.get(&3) {
        let json_blocks = extract_json_blocks(text);
        if let Some(j) = json_blocks.first() {
            tokens["spacing"] = serde_json::from_str(j).map_err(|e| ParseError::InvalidJson {
                section: 3,
                source: e,
            })?;
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
