//! Input renderers: text/number inputs, select, date, checkbox, radio, etc.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

use serde_json::Value;
use std::fmt::Write as _;

pub(super) fn render_text_input(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("text-input");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Text");
    let ph = schema
        .get("placeholder")
        .and_then(Value::as_str)
        .unwrap_or("");
    format!(
        r#"<div data-flint-component="text-input" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="text" name="{name}" placeholder="{ph}" class="input input-bordered w-full max-w-xs" />
</div>"#
    )
}

pub(super) fn render_number_input(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("number-input");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Number");
    let min = schema
        .get("min")
        .and_then(Value::as_i64)
        .map(|v| format!("min=\"{v}\""))
        .unwrap_or_default();
    let max = schema
        .get("max")
        .and_then(Value::as_i64)
        .map(|v| format!("max=\"{v}\""))
        .unwrap_or_default();
    format!(
        r#"<div data-flint-component="number-input" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="number" name="{name}" {min} {max} class="input input-bordered w-full max-w-xs" />
</div>"#
    )
}

pub(super) fn render_select(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("select");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Select");
    let options = schema
        .get("options")
        .and_then(Value::as_array)
        .map(|opts| {
            opts.iter()
                .filter_map(|o| {
                    o.as_str()
                        .or_else(|| o.get("label").and_then(Value::as_str))
                })
                .map(|o| format!(r#"<option value="{o}">{o}</option>"#))
                .collect::<String>()
        })
        .unwrap_or_else(|| "<option>Option 1</option><option>Option 2</option>".into());
    format!(
        r#"<div data-flint-component="select" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <select name="{name}" class="select select-bordered">{options}</select>
</div>"#
    )
}

pub(super) fn render_multi_select(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("multi-select");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Multi-select");
    format!(
        r#"<div data-flint-component="multi-select" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <select name="{name}" multiple class="select select-bordered h-32">
    <option>Option A</option><option>Option B</option><option>Option C</option>
  </select>
  <label class="label"><span class="label-text-alt">Hold Ctrl/Cmd to select multiple</span></label>
</div>"#
    )
}

pub(super) fn render_date_picker(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("date-picker");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Date");
    format!(
        r#"<div data-flint-component="date-picker" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="date" name="{name}" class="input input-bordered w-full max-w-xs" />
</div>"#
    )
}

pub(super) fn render_checkbox(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("checkbox");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Checkbox");
    let checked = if schema
        .get("checked")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "checked"
    } else {
        ""
    };
    format!(
        r#"<div data-flint-component="checkbox" class="form-control">
  <label class="label cursor-pointer gap-4">
    <span class="label-text">{label}</span>
    <input type="checkbox" name="{name}" {checked} class="checkbox" />
  </label>
</div>"#
    )
}

pub(super) fn render_radio(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("radio");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Choose");
    let opts: Vec<&str> = schema
        .get("options")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|o| o.as_str()).collect())
        .unwrap_or_else(|| vec!["Option A", "Option B"]);
    let mut items = String::new();
    for opt in &opts {
        // `write!` into a `String` is infallible; discarding the `Result` is safe.
        let _ = write!(
            items,
            r#"<label class="label cursor-pointer gap-4 justify-start">
      <input type="radio" name="{name}" value="{opt}" class="radio" />
      <span class="label-text">{opt}</span></label>"#
        );
    }
    format!(
        r#"<div data-flint-component="radio" class="form-control">
  <span class="label-text font-medium mb-2">{label}</span>
  {items}
</div>"#
    )
}

pub(super) fn render_toggle(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("toggle");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Toggle");
    let checked = if schema
        .get("checked")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "checked"
    } else {
        ""
    };
    format!(
        r#"<div data-flint-component="toggle" class="form-control">
  <label class="label cursor-pointer gap-4">
    <span class="label-text">{label}</span>
    <input type="checkbox" name="{name}" {checked} class="toggle toggle-primary" />
  </label>
</div>"#
    )
}

pub(super) fn render_textarea(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("textarea");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Message");
    let rows = schema.get("rows").and_then(Value::as_i64).unwrap_or(4);
    format!(
        r#"<div data-flint-component="textarea" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <textarea name="{name}" rows="{rows}" class="textarea textarea-bordered" placeholder="Type here…"></textarea>
</div>"#
    )
}

pub(super) fn render_file_upload(schema: &Value) -> String {
    let name = schema.get("name").and_then(Value::as_str).unwrap_or("file");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Upload file");
    let accept = schema
        .get("accept")
        .and_then(Value::as_str)
        .unwrap_or("*/*");
    format!(
        r#"<div data-flint-component="file-upload" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="file" name="{name}" accept="{accept}" class="file-input file-input-bordered w-full max-w-xs" />
</div>"#
    )
}

pub(super) fn render_search_input(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("search");
    let ph = schema
        .get("placeholder")
        .and_then(Value::as_str)
        .unwrap_or("Search…");
    format!(
        r#"<div data-flint-component="search-input" class="form-control">
  <div class="input-group">
    <input type="search" name="{name}" placeholder="{ph}" class="input input-bordered" />
    <button class="btn btn-square"><svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg></button>
  </div>
</div>"#
    )
}

pub(super) fn render_color_picker(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("color");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Color");
    let val = schema
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or("#2563eb");
    format!(
        r#"<div data-flint-component="color-picker" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="color" name="{name}" value="{val}" class="input input-bordered h-12 p-1 w-full max-w-xs" />
</div>"#
    )
}

pub(super) fn render_slider(schema: &Value) -> String {
    let name = schema
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("slider");
    let label = schema
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Range");
    let min = schema.get("min").and_then(Value::as_i64).unwrap_or(0);
    let max = schema.get("max").and_then(Value::as_i64).unwrap_or(100);
    let val = schema.get("value").and_then(Value::as_i64).unwrap_or(50);
    format!(
        r#"<div data-flint-component="slider" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span><span class="label-text-alt">{val}</span></label>
  <input type="range" name="{name}" min="{min}" max="{max}" value="{val}" class="range range-primary" />
</div>"#
    )
}
