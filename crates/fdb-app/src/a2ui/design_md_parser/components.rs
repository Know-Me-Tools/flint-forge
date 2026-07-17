//! §5 "Components" section parsing — per-component prop/CSS-var overrides.

use super::text_extract::extract_json_blocks;
use super::ComponentOverride;

/// Parse §5 component override blocks. Each component starts with a heading
/// `### <slug>` and can contain JSON blocks for `prop_defaults` and `css_vars`.
pub(super) fn parse_component_overrides(text: &str) -> Vec<ComponentOverride> {
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
    if prop_defaults == serde_json::json!({})
        && css_vars == serde_json::json!({})
        && body.trim().is_empty()
    {
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
