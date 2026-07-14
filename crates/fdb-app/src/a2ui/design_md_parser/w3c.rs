//! W3C Design Token Community Group 2024 JSON → Flint token-map conversion.

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
            result.insert(category.clone(), flatten_w3c_group(value));
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
