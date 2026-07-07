//! Minimal prompt-template renderer for Flint Ember summaries.
//!
//! Supports `{column_name}` placeholders that are replaced with the matching
//! value from a JSONB row object. Missing placeholders are left unchanged and
//! the worker logs a warning. No expression evaluation or nested rendering is
//! supported, which keeps template injection limited to literal string values.

use pgrx::prelude::*;
use pgrx::JsonB;
use serde_json::Value;

/// Render a template by replacing `{key}` placeholders with values from `ctx`.
///
/// Keys are looked up case-sensitively in the JSONB object. Values that are not
/// strings are serialized with `serde_json` and then unquoted if they were a
/// JSON string. Missing keys are left as literal `{key}` and reported via the
/// optional `missing` vector.
pub fn render(template: &str, ctx: &Value, missing: &mut Vec<String>) -> String {
    let mut output = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();

    while let Some((_start, ch)) = chars.next() {
        if ch != '{' {
            output.push(ch);
            continue;
        }

        // Find the matching `}`.
        let mut key = String::new();
        let mut closed = false;
        for (_, inner) in chars.by_ref() {
            if inner == '}' {
                closed = true;
                break;
            }
            key.push(inner);
        }

        if !closed || key.is_empty() {
            // Not a well-formed placeholder; preserve the literal text.
            output.push('{');
            output.push_str(&key);
            if closed {
                output.push('}');
            }
            continue;
        }

        match ctx.get(&key) {
            Some(Value::String(s)) => output.push_str(s),
            Some(v) => output.push_str(&v.to_string()),
            None => {
                missing.push(key.clone());
                output.push('{');
                output.push_str(&key);
                output.push('}');
            }
        }
    }

    output
}

/// SQL-exposed template renderer for use inside triggers.
#[pg_extern]
fn _render_template(template: &str, values: JsonB) -> String {
    let mut missing = Vec::new();
    let result = render(template, &values.0, &mut missing);
    if !missing.is_empty() {
        // Best-effort warning; never blocks the enqueue.
        eprintln!(
            "flint_llm: template missing placeholders: {}",
            missing.join(", ")
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_substitution() {
        let ctx = serde_json::json!({"body": "hello world"});
        assert_eq!(
            render("Summarize: {body}", &ctx, &mut vec![]),
            "Summarize: hello world"
        );
    }

    #[test]
    fn missing_placeholder_preserved() {
        let ctx = serde_json::json!({"body": "x"});
        let mut missing = Vec::new();
        assert_eq!(
            render("{body}: {missing}", &ctx, &mut missing),
            "x: {missing}"
        );
        assert_eq!(missing, vec!["missing"]);
    }

    #[test]
    fn non_string_value() {
        let ctx = serde_json::json!({"count": 42});
        assert_eq!(render("Count: {count}", &ctx, &mut vec![]), "Count: 42");
    }

    #[test]
    fn empty_template() {
        assert_eq!(render("", &Value::Null, &mut vec![]), "");
    }
}
