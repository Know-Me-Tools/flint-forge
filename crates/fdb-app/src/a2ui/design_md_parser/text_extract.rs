//! Small text-extraction helpers shared across DESIGN.md section parsers.

/// Extract fenced JSON code blocks from a section.
pub(super) fn extract_json_blocks(text: &str) -> Vec<String> {
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
pub(super) fn extract_kv_as_object(text: &str) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with('-') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().replace(' ', "_").to_lowercase();
            let value = v
                .trim()
                .trim_end_matches(',')
                .trim_matches('"')
                .trim()
                .to_owned();
            if !key.is_empty() && !value.is_empty() {
                map.insert(key, serde_json::Value::String(value));
            }
        }
    }
    serde_json::Value::Object(map)
}
