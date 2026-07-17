//! Action renderers: action bar, dropdown menu, context menu, fab, link.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

use serde_json::Value;

pub(super) fn render_action_bar(schema: &Value) -> String {
    let actions = schema
        .get("actions")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|act| act.get("label").and_then(Value::as_str))
                .map(|l| format!(r#"<button class="btn btn-sm">{l}</button>"#))
                .collect::<String>()
        })
        .unwrap_or_else(|| "<button class=\"btn btn-sm btn-primary\">Action</button>".into());
    format!(
        r#"<div data-flint-component="action-bar" class="flex gap-2 flex-wrap">{actions}</div>"#
    )
}

pub(super) fn render_dropdown_menu(schema: &Value) -> String {
    let trigger = schema
        .get("trigger_label")
        .and_then(Value::as_str)
        .unwrap_or("Options");
    let items = schema
        .get("items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|i| i.get("label").and_then(Value::as_str))
                .map(|l| format!(r#"<li><a>{l}</a></li>"#))
                .collect::<String>()
        })
        .unwrap_or_else(|| "<li><a>Item 1</a></li><li><a>Item 2</a></li>".into());
    format!(
        r#"<div data-flint-component="dropdown-menu" class="dropdown">
  <label tabindex="0" class="btn m-1">{trigger}</label>
  <ul tabindex="0" class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52">{items}</ul>
</div>"#
    )
}

pub(super) fn render_context_menu(schema: &Value) -> String {
    let items = schema
        .get("items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|i| i.get("label").and_then(Value::as_str))
                .map(|l| format!(r#"<li><a>{l}</a></li>"#))
                .collect::<String>()
        })
        .unwrap_or_else(|| "<li><a>Edit</a></li><li><a>Delete</a></li>".into());
    format!(
        r#"<div data-flint-component="context-menu" class="p-2 bg-base-100 shadow rounded-box border border-base-300 w-48">
  <p class="text-xs text-base-content/50 px-2 pb-1">Context menu</p>
  <ul class="menu p-0">{items}</ul>
</div>"#
    )
}

pub(super) fn render_fab(schema: &Value) -> String {
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Add");
    let position = schema
        .get("position")
        .and_then(Value::as_str)
        .unwrap_or("bottom-right");
    let pos_class = match position {
        "bottom-left" => "fixed bottom-6 left-6",
        "top-right" => "fixed top-6 right-6",
        "top-left" => "fixed top-6 left-6",
        _ => "fixed bottom-6 right-6",
    };
    format!(
        r#"<div data-flint-component="fab" class="relative h-20">
  <button class="btn btn-circle btn-primary {pos_class}" title="{label}">
    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
  </button>
</div>"#
    )
}

pub(super) fn render_link(schema: &Value) -> String {
    let href = schema.get("href").and_then(Value::as_str).unwrap_or("#");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Link");
    let ext = schema
        .get("external")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let target = if ext {
        " target=\"_blank\" rel=\"noopener\""
    } else {
        ""
    };
    format!(
        r#"<a data-flint-component="link" href="{href}"{target} class="link link-primary">{label}</a>"#
    )
}
