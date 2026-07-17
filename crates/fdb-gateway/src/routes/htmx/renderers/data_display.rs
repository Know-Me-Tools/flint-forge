//! Data-display renderers: table, badge, tag, avatar, stat card, timeline,
//! code block, json viewer, list, detail view.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

use super::html_escape;
use serde_json::Value;

pub(super) fn render_data_table(schema: &Value) -> String {
    let headers = schema
        .get("headers")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|h| h.as_str())
                .map(|h| format!("<th>{h}</th>"))
                .collect::<String>()
        })
        .unwrap_or_else(|| "<th>Col A</th><th>Col B</th>".into());
    format!(
        r#"<div data-flint-component="data-table" class="overflow-x-auto">
  <table class="table w-full">
    <thead><tr class="bg-base-200">{headers}</tr></thead>
    <tbody><tr class="text-base-content/50"><td colspan="99" class="text-center py-4">No rows</td></tr></tbody>
  </table>
</div>"#
    )
}

pub(super) fn render_badge(schema: &Value) -> String {
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Badge");
    let color = schema
        .get("color")
        .and_then(Value::as_str)
        .unwrap_or("primary");
    let cls = match color {
        "secondary" => "badge-secondary",
        "accent" => "badge-accent",
        "error" => "badge-error",
        "warning" => "badge-warning",
        "success" => "badge-success",
        _ => "badge-primary",
    };
    format!(r#"<span data-flint-component="badge" class="badge {cls}">{label}</span>"#)
}

pub(super) fn render_tag(schema: &Value) -> String {
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Tag");
    let dismissible = schema
        .get("dismissible")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let close = if dismissible {
        r#" <button class="btn btn-xs btn-circle btn-ghost">✕</button>"#
    } else {
        ""
    };
    format!(
        r#"<div data-flint-component="tag" class="badge badge-outline gap-1">{label}{close}</div>"#
    )
}

pub(super) fn render_avatar(schema: &Value) -> String {
    let name = schema.get("name").and_then(Value::as_str).unwrap_or("User");
    let src = schema.get("src").and_then(Value::as_str);
    let size = match schema.get("size").and_then(Value::as_str).unwrap_or("md") {
        "sm" => "w-8",
        "lg" => "w-16",
        _ => "w-12",
    };
    let initials = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    if let Some(url) = src {
        format!(
            r#"<div data-flint-component="avatar" class="avatar"><div class="{size} rounded-full"><img src="{url}" alt="{name}" /></div></div>"#
        )
    } else {
        format!(
            r#"<div data-flint-component="avatar" class="avatar placeholder"><div class="{size} rounded-full bg-primary text-primary-content"><span>{initials}</span></div></div>"#
        )
    }
}

pub(super) fn render_stat_card(schema: &Value) -> String {
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Metric");
    let value = schema.get("value").and_then(Value::as_str).unwrap_or("—");
    let delta = schema.get("delta").and_then(Value::as_str);
    let trend = schema.get("trend").and_then(Value::as_str).unwrap_or("up");
    let delta_html = delta
        .map(|d| {
            let color = if trend == "up" {
                "text-success"
            } else {
                "text-error"
            };
            format!(r#"<div class="stat-desc {color}">{d}</div>"#)
        })
        .unwrap_or_default();
    format!(
        r#"<div data-flint-component="stat-card" class="stat bg-base-100 rounded-lg shadow">
  <div class="stat-title">{label}</div>
  <div class="stat-value">{value}</div>
  {delta_html}
</div>"#
    )
}

pub(super) fn render_timeline(schema: &Value) -> String {
    let events = schema.get("events").and_then(Value::as_array)
        .map(|a| a.iter().map(|e| {
            let label = e.get("label").and_then(Value::as_str).unwrap_or("Event");
            let ts    = e.get("timestamp").and_then(Value::as_str).unwrap_or("");
            format!(r#"<li><div class="timeline-start text-xs text-base-content/50">{ts}</div><div class="timeline-middle"><div class="w-2 h-2 rounded-full bg-primary"></div></div><div class="timeline-end timeline-box">{label}</div><hr/></li>"#)
        }).collect::<String>())
        .unwrap_or_else(|| r#"<li><div class="timeline-middle"><div class="w-2 h-2 rounded-full bg-primary"></div></div><div class="timeline-end timeline-box">Event</div></li>"#.into());
    format!(
        r#"<ul data-flint-component="timeline" class="timeline timeline-vertical">{events}</ul>"#
    )
}

pub(super) fn render_code_block(schema: &Value) -> String {
    let code = schema
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("// code here");
    let lang = schema
        .get("language")
        .and_then(Value::as_str)
        .unwrap_or("text");
    let escaped = html_escape(code);
    format!(
        r#"<div data-flint-component="code-block" class="mockup-code">
  <pre data-lang="{lang}"><code>{escaped}</code></pre>
</div>"#
    )
}

pub(super) fn render_json_viewer(schema: &Value) -> String {
    let data = schema
        .get("data")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"key": "value"}));
    let pretty = serde_json::to_string_pretty(&data).unwrap_or_default();
    let escaped = html_escape(&pretty);
    format!(
        r#"<div data-flint-component="json-viewer" class="bg-base-200 rounded-lg p-4 overflow-x-auto">
  <pre class="text-sm"><code>{escaped}</code></pre>
</div>"#
    )
}

pub(super) fn render_list(schema: &Value) -> String {
    let items = schema
        .get("items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|i| i.as_str())
                .map(|i| format!("<li>{i}</li>"))
                .collect::<String>()
        })
        .unwrap_or_else(|| "<li>Item 1</li><li>Item 2</li><li>Item 3</li>".into());
    let ordered = schema
        .get("ordered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tag = if ordered { "ol" } else { "ul" };
    let cls = if ordered { "list-decimal" } else { "list-disc" };
    format!(r#"<{tag} data-flint-component="list" class="{cls} pl-5 space-y-1">{items}</{tag}>"#)
}

pub(super) fn render_detail_view(schema: &Value) -> String {
    let fields = schema.get("fields").and_then(Value::as_array)
        .map(|a| a.iter().map(|f| {
            let label = f.get("label").and_then(Value::as_str).unwrap_or("Field");
            let value = f.get("value").and_then(Value::as_str).unwrap_or("—");
            format!(r#"<div class="py-2 flex gap-4"><dt class="w-32 text-sm font-medium text-base-content/50 shrink-0">{label}</dt><dd class="text-sm">{value}</dd></div>"#)
        }).collect::<String>())
        .unwrap_or_else(|| r#"<div class="py-2 flex gap-4"><dt class="w-32 text-sm font-medium text-base-content/50">Label</dt><dd class="text-sm">Value</dd></div>"#.into());
    format!(
        r#"<dl data-flint-component="detail-view" class="divide-y divide-base-300">{fields}</dl>"#
    )
}
