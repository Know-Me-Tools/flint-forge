//! Navigation renderers: nav bar, sidebar, breadcrumb, pagination, stepper.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

use serde_json::Value;

pub(super) fn render_nav_bar(schema: &Value) -> String {
    let links = schema
        .get("links")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|l| l.get("label").and_then(Value::as_str))
                .map(|l| format!(r#"<li><a class="btn btn-ghost btn-sm">{l}</a></li>"#))
                .collect::<String>()
        })
        .unwrap_or_else(|| "<li><a class=\"btn btn-ghost btn-sm\">Home</a></li>".into());
    format!(
        r#"<nav data-flint-component="nav-bar" class="navbar bg-base-100 shadow">
  <div class="flex-1"><span class="font-bold px-4">App</span></div>
  <div class="flex-none"><ul class="menu menu-horizontal">{links}</ul></div>
</nav>"#
    )
}

pub(super) fn render_sidebar(schema: &Value) -> String {
    let items = schema
        .get("items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|i| i.get("label").and_then(Value::as_str))
                .map(|l| format!(r#"<li><a>{l}</a></li>"#))
                .collect::<String>()
        })
        .unwrap_or_else(|| "<li><a>Dashboard</a></li><li><a>Settings</a></li>".into());
    format!(
        r#"<aside data-flint-component="sidebar" class="w-48 min-h-40 bg-base-200 rounded-lg p-2">
  <ul class="menu">{items}</ul>
</aside>"#
    )
}

pub(super) fn render_breadcrumb(schema: &Value) -> String {
    let items = schema
        .get("items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|i| i.get("label").and_then(Value::as_str))
                .enumerate()
                .map(|(i, l)| {
                    if i == 0 {
                        format!("<li><a>{l}</a></li>")
                    } else {
                        format!("<li>{l}</li>")
                    }
                })
                .collect::<String>()
        })
        .unwrap_or_else(|| "<li><a>Home</a></li><li>Page</li>".into());
    format!(
        r#"<div data-flint-component="breadcrumb" class="breadcrumbs text-sm">
  <ul>{items}</ul>
</div>"#
    )
}

pub(super) fn render_pagination(schema: &Value) -> String {
    let total = schema.get("total").and_then(Value::as_i64).unwrap_or(100);
    let page = schema.get("page").and_then(Value::as_i64).unwrap_or(1);
    let size = schema
        .get("page_size")
        .and_then(Value::as_i64)
        .unwrap_or(25);
    let pages = (total + size - 1) / size;
    format!(
        r#"<div data-flint-component="pagination" class="join">
  <button class="join-item btn btn-sm">«</button>
  <button class="join-item btn btn-sm btn-active">{page}</button>
  <button class="join-item btn btn-sm">{}</button>
  <button class="join-item btn btn-sm">»</button>
</div>
<p class="text-sm text-base-content/50 mt-1">Page {page} of {pages}</p>"#,
        page + 1
    )
}

pub(super) fn render_stepper(schema: &Value) -> String {
    let steps = schema
        .get("steps")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|s| s.get("label").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["Step 1", "Step 2", "Step 3"]);
    let current = schema.get("current").and_then(Value::as_i64).unwrap_or(0) as usize;
    let items = steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let cls = if i <= current {
                "step step-primary"
            } else {
                "step"
            };
            format!(r#"<li class="{cls}">{s}</li>"#)
        })
        .collect::<String>();
    format!(r#"<ul data-flint-component="stepper" class="steps">{items}</ul>"#)
}
