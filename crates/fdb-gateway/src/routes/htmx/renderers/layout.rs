//! Layout renderers: container, row, column, grid, stack, divider, spacer, scroll area.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

use serde_json::Value;

pub(super) fn render_container(schema: &Value) -> String {
    let max_w = schema
        .get("max_width")
        .and_then(Value::as_str)
        .unwrap_or("1280px");
    let centered = schema
        .get("centered")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mx = if centered { "mx-auto" } else { "" };
    format!(
        r#"<div data-flint-component="container" class="px-4 {mx}" style="max-width:{max_w}">
  <div class="bg-base-200 rounded p-4 text-base-content/40 text-center text-sm">Container ({max_w})</div>
</div>"#
    )
}

pub(super) fn render_row(schema: &Value) -> String {
    let gap = schema
        .get("gap")
        .and_then(Value::as_str)
        .unwrap_or("var(--space-sm)");
    format!(
        r#"<div data-flint-component="row" class="flex flex-row gap-2 items-center" style="gap:{gap}">
  <div class="badge">Item 1</div><div class="badge">Item 2</div><div class="badge">Item 3</div>
</div>"#
    )
}

pub(super) fn render_column(schema: &Value) -> String {
    let gap = schema
        .get("gap")
        .and_then(Value::as_str)
        .unwrap_or("var(--space-sm)");
    format!(
        r#"<div data-flint-component="column" class="flex flex-col gap-2" style="gap:{gap}">
  <div class="badge">Item 1</div><div class="badge">Item 2</div><div class="badge">Item 3</div>
</div>"#
    )
}

pub(super) fn render_grid(schema: &Value) -> String {
    let cols = schema.get("columns").and_then(Value::as_i64).unwrap_or(3);
    let gap = schema
        .get("gap")
        .and_then(Value::as_str)
        .unwrap_or("var(--space-md)");
    format!(
        r#"<div data-flint-component="grid" class="grid gap-4" style="grid-template-columns:repeat({cols},minmax(0,1fr));gap:{gap}">
  <div class="bg-base-200 rounded p-4 text-center text-sm">1</div>
  <div class="bg-base-200 rounded p-4 text-center text-sm">2</div>
  <div class="bg-base-200 rounded p-4 text-center text-sm">3</div>
</div>"#
    )
}

pub(super) fn render_stack(schema: &Value) -> String {
    let _ = schema;
    r#"<div data-flint-component="stack" class="stack w-32">
  <div class="bg-primary text-primary-content rounded p-6 text-center">Layer 1</div>
  <div class="bg-secondary text-secondary-content rounded p-6 text-center">Layer 2</div>
  <div class="bg-accent text-accent-content rounded p-6 text-center">Layer 3</div>
</div>"#
        .to_owned()
}

pub(super) fn render_divider(schema: &Value) -> String {
    let orientation = schema
        .get("orientation")
        .and_then(Value::as_str)
        .unwrap_or("horizontal");
    let cls = if orientation == "vertical" {
        "divider divider-horizontal"
    } else {
        "divider"
    };
    format!(r#"<div data-flint-component="divider" class="{cls}"></div>"#)
}

pub(super) fn render_spacer(schema: &Value) -> String {
    let size = schema
        .get("size")
        .and_then(Value::as_str)
        .unwrap_or("var(--space-md)");
    format!(r#"<div data-flint-component="spacer" style="height:{size};min-height:1px;"></div>"#)
}

pub(super) fn render_scroll_area(schema: &Value) -> String {
    let max_h = schema
        .get("max_height")
        .and_then(Value::as_str)
        .unwrap_or("300px");
    let inner = "<br/><p class='text-base-content/30 text-xs'>Lorem ipsum…</p>".repeat(6);
    format!(
        r#"<div data-flint-component="scroll-area" class="overflow-y-auto border border-base-300 rounded-lg p-3" style="max-height:{max_h}">
  <p class="text-base-content/40 text-sm">Scrollable content area (max-height: {max_h})</p>
  {inner}
</div>"#
    )
}
