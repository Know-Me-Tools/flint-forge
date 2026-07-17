//! Domain types for the Flint A2UI component registry use-cases.
//!
//! These types are used by `fdb-app` use-cases that call
//! `flint_a2ui.resolve_components_with_overrides()` and return structured
//! component definitions to the interface layer.

/// A component definition returned by `resolve_components_with_overrides()`,
/// including any per-application and per-design-system overrides applied.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedComponent {
    /// Unique component identifier, matches `flint_a2ui.components.slug`.
    pub slug: String,
    /// The underlying primitive UI element this component renders as (e.g. `button`, `input`).
    pub primitive_type: String,
    /// Grouping used to organize components in the registry UI and SDK catalogs.
    pub category: String,
    /// JSON Schema describing the component's accepted props.
    pub schema: serde_json::Value,
    /// Human-readable description shown in the registry and generated SDK docs.
    pub description: Option<String>,
    /// Which SDK renderers (React, Flutter, HTMX) support this component.
    pub renderers: Renderers,
    /// Merged prop defaults from component_overrides (empty object if none)
    pub prop_defaults: serde_json::Value,
    /// Merged CSS variables from component_overrides (empty object if none)
    pub css_vars: serde_json::Value,
    /// Overridden React component import path (None = use SDK default)
    pub react_component: Option<String>,
    /// Overridden Flutter widget class name (None = use SDK default)
    pub flutter_widget: Option<String>,
    /// Overridden Askama template path (None = use SDK default)
    pub htmx_template: Option<String>,
}

/// Which SDK renderers support a given component.
///
/// A component with `flutter: false` is excluded from the Flutter SDK catalog.
/// A component with `htmx: false` is excluded from the HTMX renderer template set.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Renderers {
    /// Whether the React SDK renders this component.
    pub react: bool,
    /// Whether the Flutter SDK renders this component.
    pub flutter: bool,
    /// Whether the HTMX/Askama renderer renders this component.
    pub htmx: bool,
}

impl Default for Renderers {
    fn default() -> Self {
        Self {
            react: true,
            flutter: true,
            htmx: true,
        }
    }
}

/// A single W3C Design Tokens Community Group 2024 token value.
///
/// Stored in `design_systems.tokens` as nested JSONB. The `$value` and `$type`
/// fields correspond to the W3C DTCG 2024 format.
///
/// Reference: <https://design-tokens.org/schema/2024>
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DesignToken {
    /// The token's literal value (e.g. a color, dimension, or font-family string).
    #[serde(rename = "$value")]
    pub value: String,
    /// The DTCG token type (e.g. `"color"`, `"dimension"`, `"fontFamily"`).
    #[serde(rename = "$type")]
    pub token_type: String,
}

/// The full design token map for a design system, keyed by group then token name.
///
/// Example:
/// ```json
/// {
///   "color": {
///     "primary": { "$value": "oklch(68% 0.21 250)", "$type": "color" }
///   }
/// }
/// ```
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DesignTokenMap(pub serde_json::Value);

impl DesignTokenMap {
    /// Build an empty token map (an empty JSON object), used as the default
    /// when a design system has not yet imported any tokens.
    #[must_use]
    pub fn empty() -> Self {
        Self(serde_json::Value::Object(serde_json::Map::default()))
    }
}
