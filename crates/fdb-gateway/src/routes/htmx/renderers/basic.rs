//! Existing renderers (unchanged): data-grid, form, button, text, card, tabs.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

use serde_json::Value;
use std::fmt::Write as _;

pub(in crate::routes::htmx) fn render_data_grid(schema: &Value) -> String {
    let columns: Vec<String> = schema.get("columns").and_then(Value::as_array).map_or_else(
        || vec!["Column A".into(), "Column B".into(), "Column C".into()],
        |cols| {
            cols.iter()
                .filter_map(|c| {
                    c.as_str()
                        .map(str::to_owned)
                        .or_else(|| c.get("name").and_then(Value::as_str).map(str::to_owned))
                })
                .collect()
        },
    );
    let mut header = String::new();
    let mut body = String::new();
    for col in &columns {
        // `write!` into a `String` is infallible; discarding the `Result` is safe.
        let _ = write!(header, "<th class='px-4 py-2 text-left'>{col}</th>");
        body.push_str("<td class='px-4 py-2 border-t border-base-300'>Row value</td>");
    }
    format!(
        r#"<div data-flint-component="data-grid" class="overflow-x-auto">
  <table class="table table-zebra w-full">
    <thead><tr class="bg-base-200">{header}</tr></thead>
    <tbody><tr>{body}</tr><tr>{body}</tr><tr>{body}</tr></tbody>
  </table>
</div>"#
    )
}

pub(in crate::routes::htmx) fn render_form(schema: &Value) -> String {
    let fields: Vec<String> = schema.get("fields").and_then(Value::as_array).map_or_else(
        || vec!["name".into(), "email".into()],
        |flds| {
            flds.iter()
                .filter_map(|f| f.get("name").and_then(Value::as_str).map(str::to_owned))
                .collect()
        },
    );
    let mut fields_html = String::new();
    for field in &fields {
        let ft = if field.contains("email") {
            "email"
        } else {
            "text"
        };
        let label = field.replace('_', " ");
        // `write!` into a `String` is infallible; discarding the `Result` is safe.
        let _ = write!(
            fields_html,
            r#"
      <div class="form-control mb-3">
        <label class="label" for="{field}"><span class="label-text capitalize">{label}</span></label>
        <input type="{ft}" id="{field}" name="{field}" class="input input-bordered w-full" data-flint-field="{field}" />
      </div>"#
        );
    }
    format!(
        r##"<form data-flint-component="form" hx-post="/api/public/example" hx-target="#form-result" hx-swap="innerHTML" class="card bg-base-100 shadow p-6 max-w-lg space-y-2">
  {fields_html}
  <button type="submit" class="btn btn-primary w-full">Submit</button>
</form>
<div id="form-result"></div>"##
    )
}

pub(in crate::routes::htmx) fn render_button(schema: &Value) -> String {
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Button");
    let btn_class = match schema
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or("primary")
    {
        "secondary" => "btn-secondary",
        "outline" => "btn-outline",
        "ghost" => "btn-ghost",
        _ => "btn-primary",
    };
    format!(r#"<button data-flint-component="button" class="btn {btn_class}">{label}</button>"#)
}

pub(in crate::routes::htmx) fn render_text(schema: &Value) -> String {
    let content = schema
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("Sample text content");
    let (tag, class) = match schema
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or("body")
    {
        "h1" => ("h1", "text-4xl font-bold"),
        "h2" => ("h2", "text-3xl font-semibold"),
        "h3" => ("h3", "text-2xl font-semibold"),
        "caption" => ("p", "text-sm text-base-content/60"),
        _ => ("p", "text-base"),
    };
    format!(r#"<{tag} data-flint-component="text" class="{class}">{content}</{tag}>"#)
}

pub(in crate::routes::htmx) fn render_card(schema: &Value) -> String {
    let title = schema
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Card Title");
    let body = schema
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("Card body content.");
    format!(
        r#"<div data-flint-component="card" class="card bg-base-100 shadow border border-base-300">
  <div class="card-body"><h3 class="card-title">{title}</h3><p class="text-base-content/70">{body}</p></div>
</div>"#
    )
}

pub(in crate::routes::htmx) fn render_tabs(schema: &Value) -> String {
    let tabs: Vec<String> = schema.get("tabs").and_then(Value::as_array).map_or_else(
        || vec!["Tab 1".into(), "Tab 2".into()],
        |tbs| {
            tbs.iter()
                .filter_map(|t| {
                    t.as_str()
                        .map(str::to_owned)
                        .or_else(|| t.get("label").and_then(Value::as_str).map(str::to_owned))
                })
                .collect()
        },
    );
    let mut buttons = String::new();
    let mut panels = String::new();
    for (i, tab) in tabs.iter().enumerate() {
        let active = if i == 0 { "tab-active" } else { "" };
        let display = if i == 0 { "" } else { "hidden" };
        // `write!` into a `String` is infallible; discarding the `Result` is safe (both sites below).
        let _ = write!(buttons, r#"<a class="tab tab-lifted {active}">{tab}</a>"#);
        let _ = write!(
            panels,
            r#"<div class="tab-content {display} p-4"><p class="text-base-content/60">Content for {tab}</p></div>"#
        );
    }
    format!(
        r#"<div data-flint-component="tabs" role="tablist" class="tabs tabs-boxed">{buttons}</div><div>{panels}</div>"#
    )
}
