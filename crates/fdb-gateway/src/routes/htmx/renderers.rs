//! HTMX component HTML renderers.
//!
//! Each `render_<slug>()` function returns a self-contained HTML fragment using
//! DaisyUI classes and `data-flint-component="<slug>"` attributes. The dispatch
//! function `render_component_html()` routes to the correct renderer by slug,
//! falling back to `render_generic()` for slugs without dedicated renderers.
#![forbid(unsafe_code)]
// Renderer functions build strings via iterators and option chains. The patterns
// flagged by these lints are intentional for readability in mechanical renderer code.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

use std::fmt::Write as _;
use serde_json::Value;

// ─── Dispatch ────────────────────────────────────────────────────────────────

/// Route to a slug-specific renderer, falling back to the generic JSON card.
pub(super) fn render_component_html(slug: &str, schema: &Value, description: Option<&str>) -> String {
    match slug {
        // ── Existing renderers ────────────────────────────────────────────
        "data-grid"              => render_data_grid(schema),
        "form" | "form-view"     => render_form(schema),
        "button"                 => render_button(schema),
        "text" | "text-block"    => render_text(schema),
        "card"                   => render_card(schema),
        "tabs"                   => render_tabs(schema),
        // ── Input ─────────────────────────────────────────────────────────
        "text-input"             => render_text_input(schema),
        "number-input"           => render_number_input(schema),
        "select"                 => render_select(schema),
        "multi-select"           => render_multi_select(schema),
        "date-picker"            => render_date_picker(schema),
        "checkbox"               => render_checkbox(schema),
        "radio"                  => render_radio(schema),
        "toggle"                 => render_toggle(schema),
        "textarea"               => render_textarea(schema),
        "file-upload"            => render_file_upload(schema),
        "search-input"           => render_search_input(schema),
        "color-picker"           => render_color_picker(schema),
        "slider"                 => render_slider(schema),
        // ── Action ────────────────────────────────────────────────────────
        "action-bar"             => render_action_bar(schema),
        "dropdown-menu"          => render_dropdown_menu(schema),
        "context-menu"           => render_context_menu(schema),
        "fab"                    => render_fab(schema),
        "link"                   => render_link(schema),
        // ── Navigation ───────────────────────────────────────────────────
        "nav-bar"                => render_nav_bar(schema),
        "sidebar"                => render_sidebar(schema),
        "breadcrumb"             => render_breadcrumb(schema),
        "pagination"             => render_pagination(schema),
        "stepper"                => render_stepper(schema),
        // ── Feedback ─────────────────────────────────────────────────────
        "alert"                  => render_alert(schema),
        "toast"                  => render_toast(schema),
        "modal"                  => render_modal(schema),
        "dialog"                 => render_dialog(schema),
        "loading-spinner"        => render_loading_spinner(schema),
        "progress-bar"           => render_progress_bar(schema),
        "empty-state"            => render_empty_state(schema),
        "error-boundary"         => render_error_boundary(schema),
        // ── Data-display ──────────────────────────────────────────────────
        "data-table"             => render_data_table(schema),
        "badge"                  => render_badge(schema),
        "tag"                    => render_tag(schema),
        "avatar"                 => render_avatar(schema),
        "stat-card"              => render_stat_card(schema),
        "timeline"               => render_timeline(schema),
        "code-block"             => render_code_block(schema),
        "json-viewer"            => render_json_viewer(schema),
        "list"                   => render_list(schema),
        "detail-view"            => render_detail_view(schema),
        // ── Layout ───────────────────────────────────────────────────────
        "container"              => render_container(schema),
        "row"                    => render_row(schema),
        "column"                 => render_column(schema),
        "grid"                   => render_grid(schema),
        "stack"                  => render_stack(schema),
        "divider"                => render_divider(schema),
        "spacer"                 => render_spacer(schema),
        "scroll-area"            => render_scroll_area(schema),
        // ── Fallback ─────────────────────────────────────────────────────
        _                        => render_generic(slug, schema, description),
    }
}

// ─── Existing renderers (unchanged) ─────────────────────────────────────────

pub(super) fn render_data_grid(schema: &Value) -> String {
    let columns: Vec<String> = schema
        .get("columns")
        .and_then(Value::as_array)
        .map_or_else(
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

pub(super) fn render_form(schema: &Value) -> String {
    let fields: Vec<String> = schema
        .get("fields")
        .and_then(Value::as_array)
        .map_or_else(
            || vec!["name".into(), "email".into()],
            |flds| flds.iter().filter_map(|f| f.get("name").and_then(Value::as_str).map(str::to_owned)).collect(),
        );
    let mut fields_html = String::new();
    for field in &fields {
        let ft = if field.contains("email") { "email" } else { "text" };
        let label = field.replace('_', " ");
        let _ = write!(fields_html, r#"
      <div class="form-control mb-3">
        <label class="label" for="{field}"><span class="label-text capitalize">{label}</span></label>
        <input type="{ft}" id="{field}" name="{field}" class="input input-bordered w-full" data-flint-field="{field}" />
      </div>"#);
    }
    format!(r##"<form data-flint-component="form" hx-post="/api/public/example" hx-target="#form-result" hx-swap="innerHTML" class="card bg-base-100 shadow p-6 max-w-lg space-y-2">
  {fields_html}
  <button type="submit" class="btn btn-primary w-full">Submit</button>
</form>
<div id="form-result"></div>"##)
}

pub(super) fn render_button(schema: &Value) -> String {
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Button");
    let btn_class = match schema.get("variant").and_then(Value::as_str).unwrap_or("primary") {
        "secondary" => "btn-secondary", "outline" => "btn-outline", "ghost" => "btn-ghost", _ => "btn-primary",
    };
    format!(r#"<button data-flint-component="button" class="btn {btn_class}">{label}</button>"#)
}

pub(super) fn render_text(schema: &Value) -> String {
    let content = schema.get("content").and_then(Value::as_str).unwrap_or("Sample text content");
    let (tag, class) = match schema.get("variant").and_then(Value::as_str).unwrap_or("body") {
        "h1" => ("h1", "text-4xl font-bold"), "h2" => ("h2", "text-3xl font-semibold"),
        "h3" => ("h3", "text-2xl font-semibold"), "caption" => ("p", "text-sm text-base-content/60"),
        _ => ("p", "text-base"),
    };
    format!(r#"<{tag} data-flint-component="text" class="{class}">{content}</{tag}>"#)
}

pub(super) fn render_card(schema: &Value) -> String {
    let title = schema.get("title").and_then(Value::as_str).unwrap_or("Card Title");
    let body  = schema.get("body").and_then(Value::as_str).unwrap_or("Card body content.");
    format!(r#"<div data-flint-component="card" class="card bg-base-100 shadow border border-base-300">
  <div class="card-body"><h3 class="card-title">{title}</h3><p class="text-base-content/70">{body}</p></div>
</div>"#)
}

pub(super) fn render_tabs(schema: &Value) -> String {
    let tabs: Vec<String> = schema.get("tabs").and_then(Value::as_array)
        .map_or_else(|| vec!["Tab 1".into(), "Tab 2".into()],
            |tbs| tbs.iter().filter_map(|t| t.as_str().map(str::to_owned).or_else(|| t.get("label").and_then(Value::as_str).map(str::to_owned))).collect());
    let mut buttons = String::new();
    let mut panels  = String::new();
    for (i, tab) in tabs.iter().enumerate() {
        let active  = if i == 0 { "tab-active" } else { "" };
        let display = if i == 0 { "" } else { "hidden" };
        let _ = write!(buttons, r#"<a class="tab tab-lifted {active}">{tab}</a>"#);
        let _ = write!(panels,  r#"<div class="tab-content {display} p-4"><p class="text-base-content/60">Content for {tab}</p></div>"#);
    }
    format!(r#"<div data-flint-component="tabs" role="tablist" class="tabs tabs-boxed">{buttons}</div><div>{panels}</div>"#)
}

// ─── Input renderers ─────────────────────────────────────────────────────────

fn render_text_input(schema: &Value) -> String {
    let name  = schema.get("name").and_then(Value::as_str).unwrap_or("text-input");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Text");
    let ph    = schema.get("placeholder").and_then(Value::as_str).unwrap_or("");
    format!(r#"<div data-flint-component="text-input" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="text" name="{name}" placeholder="{ph}" class="input input-bordered w-full max-w-xs" />
</div>"#)
}

fn render_number_input(schema: &Value) -> String {
    let name  = schema.get("name").and_then(Value::as_str).unwrap_or("number-input");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Number");
    let min   = schema.get("min").and_then(Value::as_i64).map(|v| format!("min=\"{v}\"")).unwrap_or_default();
    let max   = schema.get("max").and_then(Value::as_i64).map(|v| format!("max=\"{v}\"")).unwrap_or_default();
    format!(r#"<div data-flint-component="number-input" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="number" name="{name}" {min} {max} class="input input-bordered w-full max-w-xs" />
</div>"#)
}

fn render_select(schema: &Value) -> String {
    let name    = schema.get("name").and_then(Value::as_str).unwrap_or("select");
    let label   = schema.get("label").and_then(Value::as_str).unwrap_or("Select");
    let options = schema.get("options").and_then(Value::as_array)
        .map(|opts| opts.iter()
            .filter_map(|o| o.as_str().or_else(|| o.get("label").and_then(Value::as_str)))
            .map(|o| format!(r#"<option value="{o}">{o}</option>"#))
            .collect::<String>())
        .unwrap_or_else(|| "<option>Option 1</option><option>Option 2</option>".into());
    format!(r#"<div data-flint-component="select" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <select name="{name}" class="select select-bordered">{options}</select>
</div>"#)
}

fn render_multi_select(schema: &Value) -> String {
    let name  = schema.get("name").and_then(Value::as_str).unwrap_or("multi-select");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Multi-select");
    format!(r#"<div data-flint-component="multi-select" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <select name="{name}" multiple class="select select-bordered h-32">
    <option>Option A</option><option>Option B</option><option>Option C</option>
  </select>
  <label class="label"><span class="label-text-alt">Hold Ctrl/Cmd to select multiple</span></label>
</div>"#)
}

fn render_date_picker(schema: &Value) -> String {
    let name  = schema.get("name").and_then(Value::as_str).unwrap_or("date-picker");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Date");
    format!(r#"<div data-flint-component="date-picker" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="date" name="{name}" class="input input-bordered w-full max-w-xs" />
</div>"#)
}

fn render_checkbox(schema: &Value) -> String {
    let name    = schema.get("name").and_then(Value::as_str).unwrap_or("checkbox");
    let label   = schema.get("label").and_then(Value::as_str).unwrap_or("Checkbox");
    let checked = if schema.get("checked").and_then(Value::as_bool).unwrap_or(false) { "checked" } else { "" };
    format!(r#"<div data-flint-component="checkbox" class="form-control">
  <label class="label cursor-pointer gap-4">
    <span class="label-text">{label}</span>
    <input type="checkbox" name="{name}" {checked} class="checkbox" />
  </label>
</div>"#)
}

fn render_radio(schema: &Value) -> String {
    let name  = schema.get("name").and_then(Value::as_str).unwrap_or("radio");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Choose");
    let opts: Vec<&str> = schema.get("options").and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|o| o.as_str()).collect())
        .unwrap_or_else(|| vec!["Option A", "Option B"]);
    let mut items = String::new();
    for opt in &opts {
        let _ = write!(items, r#"<label class="label cursor-pointer gap-4 justify-start">
      <input type="radio" name="{name}" value="{opt}" class="radio" />
      <span class="label-text">{opt}</span></label>"#);
    }
    format!(r#"<div data-flint-component="radio" class="form-control">
  <span class="label-text font-medium mb-2">{label}</span>
  {items}
</div>"#)
}

fn render_toggle(schema: &Value) -> String {
    let name    = schema.get("name").and_then(Value::as_str).unwrap_or("toggle");
    let label   = schema.get("label").and_then(Value::as_str).unwrap_or("Toggle");
    let checked = if schema.get("checked").and_then(Value::as_bool).unwrap_or(false) { "checked" } else { "" };
    format!(r#"<div data-flint-component="toggle" class="form-control">
  <label class="label cursor-pointer gap-4">
    <span class="label-text">{label}</span>
    <input type="checkbox" name="{name}" {checked} class="toggle toggle-primary" />
  </label>
</div>"#)
}

fn render_textarea(schema: &Value) -> String {
    let name  = schema.get("name").and_then(Value::as_str).unwrap_or("textarea");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Message");
    let rows  = schema.get("rows").and_then(Value::as_i64).unwrap_or(4);
    format!(r#"<div data-flint-component="textarea" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <textarea name="{name}" rows="{rows}" class="textarea textarea-bordered" placeholder="Type here…"></textarea>
</div>"#)
}

fn render_file_upload(schema: &Value) -> String {
    let name   = schema.get("name").and_then(Value::as_str).unwrap_or("file");
    let label  = schema.get("label").and_then(Value::as_str).unwrap_or("Upload file");
    let accept = schema.get("accept").and_then(Value::as_str).unwrap_or("*/*");
    format!(r#"<div data-flint-component="file-upload" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="file" name="{name}" accept="{accept}" class="file-input file-input-bordered w-full max-w-xs" />
</div>"#)
}

fn render_search_input(schema: &Value) -> String {
    let name = schema.get("name").and_then(Value::as_str).unwrap_or("search");
    let ph   = schema.get("placeholder").and_then(Value::as_str).unwrap_or("Search…");
    format!(r#"<div data-flint-component="search-input" class="form-control">
  <div class="input-group">
    <input type="search" name="{name}" placeholder="{ph}" class="input input-bordered" />
    <button class="btn btn-square"><svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg></button>
  </div>
</div>"#)
}

fn render_color_picker(schema: &Value) -> String {
    let name  = schema.get("name").and_then(Value::as_str).unwrap_or("color");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Color");
    let val   = schema.get("value").and_then(Value::as_str).unwrap_or("#2563eb");
    format!(r#"<div data-flint-component="color-picker" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span></label>
  <input type="color" name="{name}" value="{val}" class="input input-bordered h-12 p-1 w-full max-w-xs" />
</div>"#)
}

fn render_slider(schema: &Value) -> String {
    let name  = schema.get("name").and_then(Value::as_str).unwrap_or("slider");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Range");
    let min   = schema.get("min").and_then(Value::as_i64).unwrap_or(0);
    let max   = schema.get("max").and_then(Value::as_i64).unwrap_or(100);
    let val   = schema.get("value").and_then(Value::as_i64).unwrap_or(50);
    format!(r#"<div data-flint-component="slider" class="form-control w-full max-w-xs">
  <label class="label"><span class="label-text">{label}</span><span class="label-text-alt">{val}</span></label>
  <input type="range" name="{name}" min="{min}" max="{max}" value="{val}" class="range range-primary" />
</div>"#)
}

// ─── Action renderers ────────────────────────────────────────────────────────

fn render_action_bar(schema: &Value) -> String {
    let actions = schema.get("actions").and_then(Value::as_array)
        .map(|a| a.iter()
            .filter_map(|act| act.get("label").and_then(Value::as_str))
            .map(|l| format!(r#"<button class="btn btn-sm">{l}</button>"#))
            .collect::<String>())
        .unwrap_or_else(|| "<button class=\"btn btn-sm btn-primary\">Action</button>".into());
    format!(r#"<div data-flint-component="action-bar" class="flex gap-2 flex-wrap">{actions}</div>"#)
}

fn render_dropdown_menu(schema: &Value) -> String {
    let trigger = schema.get("trigger_label").and_then(Value::as_str).unwrap_or("Options");
    let items = schema.get("items").and_then(Value::as_array)
        .map(|a| a.iter()
            .filter_map(|i| i.get("label").and_then(Value::as_str))
            .map(|l| format!(r#"<li><a>{l}</a></li>"#))
            .collect::<String>())
        .unwrap_or_else(|| "<li><a>Item 1</a></li><li><a>Item 2</a></li>".into());
    format!(r#"<div data-flint-component="dropdown-menu" class="dropdown">
  <label tabindex="0" class="btn m-1">{trigger}</label>
  <ul tabindex="0" class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52">{items}</ul>
</div>"#)
}

fn render_context_menu(schema: &Value) -> String {
    let items = schema.get("items").and_then(Value::as_array)
        .map(|a| a.iter()
            .filter_map(|i| i.get("label").and_then(Value::as_str))
            .map(|l| format!(r#"<li><a>{l}</a></li>"#))
            .collect::<String>())
        .unwrap_or_else(|| "<li><a>Edit</a></li><li><a>Delete</a></li>".into());
    format!(r#"<div data-flint-component="context-menu" class="p-2 bg-base-100 shadow rounded-box border border-base-300 w-48">
  <p class="text-xs text-base-content/50 px-2 pb-1">Context menu</p>
  <ul class="menu p-0">{items}</ul>
</div>"#)
}

fn render_fab(schema: &Value) -> String {
    let label    = schema.get("label").and_then(Value::as_str).unwrap_or("Add");
    let position = schema.get("position").and_then(Value::as_str).unwrap_or("bottom-right");
    let pos_class = match position {
        "bottom-left"  => "fixed bottom-6 left-6",
        "top-right"    => "fixed top-6 right-6",
        "top-left"     => "fixed top-6 left-6",
        _              => "fixed bottom-6 right-6",
    };
    format!(r#"<div data-flint-component="fab" class="relative h-20">
  <button class="btn btn-circle btn-primary {pos_class}" title="{label}">
    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
  </button>
</div>"#)
}

fn render_link(schema: &Value) -> String {
    let href  = schema.get("href").and_then(Value::as_str).unwrap_or("#");
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Link");
    let ext   = schema.get("external").and_then(Value::as_bool).unwrap_or(false);
    let target = if ext { " target=\"_blank\" rel=\"noopener\"" } else { "" };
    format!(r#"<a data-flint-component="link" href="{href}"{target} class="link link-primary">{label}</a>"#)
}

// ─── Navigation renderers ────────────────────────────────────────────────────

fn render_nav_bar(schema: &Value) -> String {
    let links = schema.get("links").and_then(Value::as_array)
        .map(|a| a.iter()
            .filter_map(|l| l.get("label").and_then(Value::as_str))
            .map(|l| format!(r#"<li><a class="btn btn-ghost btn-sm">{l}</a></li>"#))
            .collect::<String>())
        .unwrap_or_else(|| "<li><a class=\"btn btn-ghost btn-sm\">Home</a></li>".into());
    format!(r#"<nav data-flint-component="nav-bar" class="navbar bg-base-100 shadow">
  <div class="flex-1"><span class="font-bold px-4">App</span></div>
  <div class="flex-none"><ul class="menu menu-horizontal">{links}</ul></div>
</nav>"#)
}

fn render_sidebar(schema: &Value) -> String {
    let items = schema.get("items").and_then(Value::as_array)
        .map(|a| a.iter()
            .filter_map(|i| i.get("label").and_then(Value::as_str))
            .map(|l| format!(r#"<li><a>{l}</a></li>"#))
            .collect::<String>())
        .unwrap_or_else(|| "<li><a>Dashboard</a></li><li><a>Settings</a></li>".into());
    format!(r#"<aside data-flint-component="sidebar" class="w-48 min-h-40 bg-base-200 rounded-lg p-2">
  <ul class="menu">{items}</ul>
</aside>"#)
}

fn render_breadcrumb(schema: &Value) -> String {
    let items = schema.get("items").and_then(Value::as_array)
        .map(|a| a.iter()
            .filter_map(|i| i.get("label").and_then(Value::as_str))
            .enumerate()
            .map(|(i, l)| if i == 0 { format!("<li><a>{l}</a></li>") } else { format!("<li>{l}</li>") })
            .collect::<String>())
        .unwrap_or_else(|| "<li><a>Home</a></li><li>Page</li>".into());
    format!(r#"<div data-flint-component="breadcrumb" class="breadcrumbs text-sm">
  <ul>{items}</ul>
</div>"#)
}

fn render_pagination(schema: &Value) -> String {
    let total = schema.get("total").and_then(Value::as_i64).unwrap_or(100);
    let page  = schema.get("page").and_then(Value::as_i64).unwrap_or(1);
    let size  = schema.get("page_size").and_then(Value::as_i64).unwrap_or(25);
    let pages = (total + size - 1) / size;
    format!(r#"<div data-flint-component="pagination" class="join">
  <button class="join-item btn btn-sm">«</button>
  <button class="join-item btn btn-sm btn-active">{page}</button>
  <button class="join-item btn btn-sm">{}</button>
  <button class="join-item btn btn-sm">»</button>
</div>
<p class="text-sm text-base-content/50 mt-1">Page {page} of {pages}</p>"#, page + 1)
}

fn render_stepper(schema: &Value) -> String {
    let steps = schema.get("steps").and_then(Value::as_array)
        .map(|a| a.iter()
            .filter_map(|s| s.get("label").and_then(Value::as_str))
            .collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["Step 1", "Step 2", "Step 3"]);
    let current = schema.get("current").and_then(Value::as_i64).unwrap_or(0) as usize;
    let items = steps.iter().enumerate().map(|(i, s)| {
        let cls = if i <= current { "step step-primary" } else { "step" };
        format!(r#"<li class="{cls}">{s}</li>"#)
    }).collect::<String>();
    format!(r#"<ul data-flint-component="stepper" class="steps">{items}</ul>"#)
}

// ─── Feedback renderers ──────────────────────────────────────────────────────

fn render_alert(schema: &Value) -> String {
    let msg = schema.get("message").and_then(Value::as_str).unwrap_or("Alert message");
    let variant = schema.get("variant").and_then(Value::as_str).unwrap_or("info");
    let cls = match variant { "success" => "alert-success", "warning" => "alert-warning", "error" => "alert-error", _ => "alert-info" };
    format!(r#"<div data-flint-component="alert" class="alert {cls}">
  <span>{msg}</span>
</div>"#)
}

fn render_toast(schema: &Value) -> String {
    let msg = schema.get("message").and_then(Value::as_str).unwrap_or("Toast notification");
    let variant = schema.get("variant").and_then(Value::as_str).unwrap_or("success");
    let cls = match variant { "error" => "alert-error", "warning" => "alert-warning", "info" => "alert-info", _ => "alert-success" };
    format!(r#"<div data-flint-component="toast" class="toast toast-end">
  <div class="alert {cls}"><span>{msg}</span></div>
</div>"#)
}

fn render_modal(schema: &Value) -> String {
    let title = schema.get("title").and_then(Value::as_str).unwrap_or("Modal");
    let size = match schema.get("size").and_then(Value::as_str).unwrap_or("md") {
        "sm" => "max-w-sm", "lg" => "max-w-2xl", "xl" => "max-w-4xl", _ => "max-w-lg",
    };
    format!(r#"<div data-flint-component="modal" class="mockup-window border border-base-300 {size}">
  <div class="p-4 bg-base-100">
    <h3 class="font-bold text-lg mb-2">{title}</h3>
    <p class="py-4 text-base-content/70">Modal content goes here.</p>
    <div class="modal-action"><button class="btn btn-ghost btn-sm">Close</button><button class="btn btn-primary btn-sm">Confirm</button></div>
  </div>
</div>"#)
}

fn render_dialog(schema: &Value) -> String {
    let title   = schema.get("title").and_then(Value::as_str).unwrap_or("Are you sure?");
    let message = schema.get("message").and_then(Value::as_str).unwrap_or("This action cannot be undone.");
    let confirm = schema.get("confirm_label").and_then(Value::as_str).unwrap_or("Confirm");
    let cancel  = schema.get("cancel_label").and_then(Value::as_str).unwrap_or("Cancel");
    format!(r#"<div data-flint-component="dialog" class="card bg-base-100 shadow w-96">
  <div class="card-body">
    <h2 class="card-title">{title}</h2>
    <p class="text-base-content/70">{message}</p>
    <div class="card-actions justify-end mt-4">
      <button class="btn btn-ghost btn-sm">{cancel}</button>
      <button class="btn btn-error btn-sm">{confirm}</button>
    </div>
  </div>
</div>"#)
}

fn render_loading_spinner(schema: &Value) -> String {
    let size = match schema.get("size").and_then(Value::as_str).unwrap_or("md") {
        "sm" => "loading-sm", "lg" => "loading-lg", "xl" => "loading-xl", _ => "loading-md",
    };
    let label = schema.get("label").and_then(Value::as_str);
    let label_html = label.map(|l| format!(r#"<span class="text-sm text-base-content/60">{l}</span>"#)).unwrap_or_default();
    format!(r#"<div data-flint-component="loading-spinner" class="flex items-center gap-3">
  <span class="loading loading-spinner {size}"></span>
  {label_html}
</div>"#)
}

fn render_progress_bar(schema: &Value) -> String {
    let value        = schema.get("value").and_then(Value::as_i64).unwrap_or(60);
    let max          = schema.get("max").and_then(Value::as_i64).unwrap_or(100);
    let indeterminate = schema.get("indeterminate").and_then(Value::as_bool).unwrap_or(false);
    let val_attr     = if indeterminate { String::new() } else { format!("value=\"{value}\" max=\"{max}\"") };
    format!(r#"<div data-flint-component="progress-bar" class="w-full">
  <progress class="progress progress-primary w-full" {val_attr}></progress>
  {}
</div>"#, if indeterminate { String::new() } else { format!("<p class=\"text-sm text-right text-base-content/50\">{value}%</p>") })
}

fn render_empty_state(schema: &Value) -> String {
    let title  = schema.get("title").and_then(Value::as_str).unwrap_or("No items found");
    let desc   = schema.get("description").and_then(Value::as_str).unwrap_or("");
    let action = schema.get("action").and_then(|a| a.get("label")).and_then(Value::as_str);
    let btn    = action.map(|l| format!(r#"<button class="btn btn-primary btn-sm mt-3">{l}</button>"#)).unwrap_or_default();
    format!(r#"<div data-flint-component="empty-state" class="flex flex-col items-center justify-center p-12 text-center">
  <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 text-base-content/20 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"/></svg>
  <h3 class="text-lg font-medium">{title}</h3>
  <p class="text-base-content/50 text-sm mt-1">{desc}</p>
  {btn}
</div>"#)
}

fn render_error_boundary(schema: &Value) -> String {
    let msg   = schema.get("message").and_then(Value::as_str).unwrap_or("Something went wrong.");
    let retry = schema.get("retry_label").and_then(Value::as_str).unwrap_or("Try again");
    format!(r#"<div data-flint-component="error-boundary" class="alert alert-error shadow-lg">
  <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 shrink-0 stroke-current" fill="none" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
  <span>{msg}</span>
  <button class="btn btn-sm btn-ghost">{retry}</button>
</div>"#)
}

// ─── Data-display renderers ──────────────────────────────────────────────────

fn render_data_table(schema: &Value) -> String {
    let headers = schema.get("headers").and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|h| h.as_str()).map(|h| format!("<th>{h}</th>")).collect::<String>())
        .unwrap_or_else(|| "<th>Col A</th><th>Col B</th>".into());
    format!(r#"<div data-flint-component="data-table" class="overflow-x-auto">
  <table class="table w-full">
    <thead><tr class="bg-base-200">{headers}</tr></thead>
    <tbody><tr class="text-base-content/50"><td colspan="99" class="text-center py-4">No rows</td></tr></tbody>
  </table>
</div>"#)
}

fn render_badge(schema: &Value) -> String {
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Badge");
    let color = schema.get("color").and_then(Value::as_str).unwrap_or("primary");
    let cls = match color { "secondary" => "badge-secondary", "accent" => "badge-accent", "error" => "badge-error", "warning" => "badge-warning", "success" => "badge-success", _ => "badge-primary" };
    format!(r#"<span data-flint-component="badge" class="badge {cls}">{label}</span>"#)
}

fn render_tag(schema: &Value) -> String {
    let label      = schema.get("label").and_then(Value::as_str).unwrap_or("Tag");
    let dismissible = schema.get("dismissible").and_then(Value::as_bool).unwrap_or(false);
    let close = if dismissible { r#" <button class="btn btn-xs btn-circle btn-ghost">✕</button>"# } else { "" };
    format!(r#"<div data-flint-component="tag" class="badge badge-outline gap-1">{label}{close}</div>"#)
}

fn render_avatar(schema: &Value) -> String {
    let name = schema.get("name").and_then(Value::as_str).unwrap_or("User");
    let src  = schema.get("src").and_then(Value::as_str);
    let size = match schema.get("size").and_then(Value::as_str).unwrap_or("md") {
        "sm" => "w-8", "lg" => "w-16", _ => "w-12",
    };
    let initials = name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase();
    if let Some(url) = src {
        format!(r#"<div data-flint-component="avatar" class="avatar"><div class="{size} rounded-full"><img src="{url}" alt="{name}" /></div></div>"#)
    } else {
        format!(r#"<div data-flint-component="avatar" class="avatar placeholder"><div class="{size} rounded-full bg-primary text-primary-content"><span>{initials}</span></div></div>"#)
    }
}

fn render_stat_card(schema: &Value) -> String {
    let label = schema.get("label").and_then(Value::as_str).unwrap_or("Metric");
    let value = schema.get("value").and_then(Value::as_str).unwrap_or("—");
    let delta = schema.get("delta").and_then(Value::as_str);
    let trend = schema.get("trend").and_then(Value::as_str).unwrap_or("up");
    let delta_html = delta.map(|d| {
        let color = if trend == "up" { "text-success" } else { "text-error" };
        format!(r#"<div class="stat-desc {color}">{d}</div>"#)
    }).unwrap_or_default();
    format!(r#"<div data-flint-component="stat-card" class="stat bg-base-100 rounded-lg shadow">
  <div class="stat-title">{label}</div>
  <div class="stat-value">{value}</div>
  {delta_html}
</div>"#)
}

fn render_timeline(schema: &Value) -> String {
    let events = schema.get("events").and_then(Value::as_array)
        .map(|a| a.iter().map(|e| {
            let label = e.get("label").and_then(Value::as_str).unwrap_or("Event");
            let ts    = e.get("timestamp").and_then(Value::as_str).unwrap_or("");
            format!(r#"<li><div class="timeline-start text-xs text-base-content/50">{ts}</div><div class="timeline-middle"><div class="w-2 h-2 rounded-full bg-primary"></div></div><div class="timeline-end timeline-box">{label}</div><hr/></li>"#)
        }).collect::<String>())
        .unwrap_or_else(|| r#"<li><div class="timeline-middle"><div class="w-2 h-2 rounded-full bg-primary"></div></div><div class="timeline-end timeline-box">Event</div></li>"#.into());
    format!(r#"<ul data-flint-component="timeline" class="timeline timeline-vertical">{events}</ul>"#)
}

fn render_code_block(schema: &Value) -> String {
    let code = schema.get("code").and_then(Value::as_str).unwrap_or("// code here");
    let lang = schema.get("language").and_then(Value::as_str).unwrap_or("text");
    let escaped = html_escape(code);
    format!(r#"<div data-flint-component="code-block" class="mockup-code">
  <pre data-lang="{lang}"><code>{escaped}</code></pre>
</div>"#)
}

fn render_json_viewer(schema: &Value) -> String {
    let data = schema.get("data").cloned().unwrap_or_else(|| serde_json::json!({"key": "value"}));
    let pretty = serde_json::to_string_pretty(&data).unwrap_or_default();
    let escaped = html_escape(&pretty);
    format!(r#"<div data-flint-component="json-viewer" class="bg-base-200 rounded-lg p-4 overflow-x-auto">
  <pre class="text-sm"><code>{escaped}</code></pre>
</div>"#)
}

fn render_list(schema: &Value) -> String {
    let items = schema.get("items").and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|i| i.as_str()).map(|i| format!("<li>{i}</li>")).collect::<String>())
        .unwrap_or_else(|| "<li>Item 1</li><li>Item 2</li><li>Item 3</li>".into());
    let ordered = schema.get("ordered").and_then(Value::as_bool).unwrap_or(false);
    let tag = if ordered { "ol" } else { "ul" };
    let cls = if ordered { "list-decimal" } else { "list-disc" };
    format!(r#"<{tag} data-flint-component="list" class="{cls} pl-5 space-y-1">{items}</{tag}>"#)
}

fn render_detail_view(schema: &Value) -> String {
    let fields = schema.get("fields").and_then(Value::as_array)
        .map(|a| a.iter().map(|f| {
            let label = f.get("label").and_then(Value::as_str).unwrap_or("Field");
            let value = f.get("value").and_then(Value::as_str).unwrap_or("—");
            format!(r#"<div class="py-2 flex gap-4"><dt class="w-32 text-sm font-medium text-base-content/50 shrink-0">{label}</dt><dd class="text-sm">{value}</dd></div>"#)
        }).collect::<String>())
        .unwrap_or_else(|| r#"<div class="py-2 flex gap-4"><dt class="w-32 text-sm font-medium text-base-content/50">Label</dt><dd class="text-sm">Value</dd></div>"#.into());
    format!(r#"<dl data-flint-component="detail-view" class="divide-y divide-base-300">{fields}</dl>"#)
}

// ─── Layout renderers ────────────────────────────────────────────────────────

fn render_container(schema: &Value) -> String {
    let max_w    = schema.get("max_width").and_then(Value::as_str).unwrap_or("1280px");
    let centered = schema.get("centered").and_then(Value::as_bool).unwrap_or(true);
    let mx       = if centered { "mx-auto" } else { "" };
    format!(r#"<div data-flint-component="container" class="px-4 {mx}" style="max-width:{max_w}">
  <div class="bg-base-200 rounded p-4 text-base-content/40 text-center text-sm">Container ({max_w})</div>
</div>"#)
}

fn render_row(schema: &Value) -> String {
    let gap = schema.get("gap").and_then(Value::as_str).unwrap_or("var(--space-sm)");
    format!(r#"<div data-flint-component="row" class="flex flex-row gap-2 items-center" style="gap:{gap}">
  <div class="badge">Item 1</div><div class="badge">Item 2</div><div class="badge">Item 3</div>
</div>"#)
}

fn render_column(schema: &Value) -> String {
    let gap = schema.get("gap").and_then(Value::as_str).unwrap_or("var(--space-sm)");
    format!(r#"<div data-flint-component="column" class="flex flex-col gap-2" style="gap:{gap}">
  <div class="badge">Item 1</div><div class="badge">Item 2</div><div class="badge">Item 3</div>
</div>"#)
}

fn render_grid(schema: &Value) -> String {
    let cols = schema.get("columns").and_then(Value::as_i64).unwrap_or(3);
    let gap  = schema.get("gap").and_then(Value::as_str).unwrap_or("var(--space-md)");
    format!(r#"<div data-flint-component="grid" class="grid gap-4" style="grid-template-columns:repeat({cols},minmax(0,1fr));gap:{gap}">
  <div class="bg-base-200 rounded p-4 text-center text-sm">1</div>
  <div class="bg-base-200 rounded p-4 text-center text-sm">2</div>
  <div class="bg-base-200 rounded p-4 text-center text-sm">3</div>
</div>"#)
}

fn render_stack(schema: &Value) -> String {
    let _ = schema;
    r#"<div data-flint-component="stack" class="stack w-32">
  <div class="bg-primary text-primary-content rounded p-6 text-center">Layer 1</div>
  <div class="bg-secondary text-secondary-content rounded p-6 text-center">Layer 2</div>
  <div class="bg-accent text-accent-content rounded p-6 text-center">Layer 3</div>
</div>"#.to_owned()
}

fn render_divider(schema: &Value) -> String {
    let orientation = schema.get("orientation").and_then(Value::as_str).unwrap_or("horizontal");
    let cls = if orientation == "vertical" { "divider divider-horizontal" } else { "divider" };
    format!(r#"<div data-flint-component="divider" class="{cls}"></div>"#)
}

fn render_spacer(schema: &Value) -> String {
    let size = schema.get("size").and_then(Value::as_str).unwrap_or("var(--space-md)");
    format!(r#"<div data-flint-component="spacer" style="height:{size};min-height:1px;"></div>"#)
}

fn render_scroll_area(schema: &Value) -> String {
    let max_h = schema.get("max_height").and_then(Value::as_str).unwrap_or("300px");
    let inner = "<br/><p class='text-base-content/30 text-xs'>Lorem ipsum…</p>".repeat(6);
    format!(r#"<div data-flint-component="scroll-area" class="overflow-y-auto border border-base-300 rounded-lg p-3" style="max-height:{max_h}">
  <p class="text-base-content/40 text-sm">Scrollable content area (max-height: {max_h})</p>
  {inner}
</div>"#)
}

// ─── Fallback renderer ───────────────────────────────────────────────────────

pub(super) fn render_generic(slug: &str, schema: &Value, description: Option<&str>) -> String {
    let pretty = serde_json::to_string_pretty(schema).unwrap_or_default();
    let desc_html = match description {
        Some(desc) if !desc.is_empty() => format!("<p class=\"text-base-content/70 mb-3\">{}</p>", html_escape(desc)),
        _ => String::new(),
    };
    format!(
        r#"<div data-flint-component="{slug}" class="card bg-base-100 shadow border border-base-300">
  <div class="card-body">
    <h3 class="card-title capitalize">{slug}</h3>
    {desc_html}
    <p class="text-sm text-base-content/50 mb-3">No dedicated HTMX renderer — showing raw schema:</p>
    <pre class="bg-base-200 p-4 rounded-lg overflow-x-auto text-sm"><code>{pretty}</code></pre>
  </div>
</div>"#,
        slug = html_escape(slug),
        desc_html = desc_html,
        pretty = html_escape(&pretty),
    )
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Minimal HTML escaper for safe insertion into text content/attributes.
pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
