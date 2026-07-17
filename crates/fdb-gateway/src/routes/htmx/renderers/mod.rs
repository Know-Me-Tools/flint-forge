//! HTMX component HTML renderers.
//!
//! Each `render_<slug>()` function returns a self-contained HTML fragment using
//! DaisyUI classes and `data-flint-component="<slug>"` attributes. The dispatch
//! function `render_component_html()` routes to the correct renderer by slug,
//! falling back to `render_generic()` for slugs without dedicated renderers.
//!
//! Renderers are grouped by kind into sibling submodules (`basic`, `input`,
//! `action`, `navigation`, `feedback`, `data_display`, `layout`); this file
//! holds only the dispatch table, the fallback renderer, and the shared
//! `html_escape` helper.
#![forbid(unsafe_code)]
// Renderer functions build strings via iterators and option chains. The patterns
// flagged by these lints are intentional for readability in mechanical renderer code.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

mod action;
mod basic;
mod data_display;
mod feedback;
mod input;
mod layout;
mod navigation;

use serde_json::Value;

pub(in crate::routes::htmx) use basic::{
    render_button, render_card, render_data_grid, render_form, render_tabs, render_text,
};

// ─── Dispatch ────────────────────────────────────────────────────────────────

/// Route to a slug-specific renderer, falling back to the generic JSON card.
pub(super) fn render_component_html(
    slug: &str,
    schema: &Value,
    description: Option<&str>,
) -> String {
    match slug {
        // ── Existing renderers ────────────────────────────────────────────
        "data-grid" => render_data_grid(schema),
        "form" | "form-view" => render_form(schema),
        "button" => render_button(schema),
        "text" | "text-block" => render_text(schema),
        "card" => render_card(schema),
        "tabs" => render_tabs(schema),
        // ── Input ─────────────────────────────────────────────────────────
        "text-input" => input::render_text_input(schema),
        "number-input" => input::render_number_input(schema),
        "select" => input::render_select(schema),
        "multi-select" => input::render_multi_select(schema),
        "date-picker" => input::render_date_picker(schema),
        "checkbox" => input::render_checkbox(schema),
        "radio" => input::render_radio(schema),
        "toggle" => input::render_toggle(schema),
        "textarea" => input::render_textarea(schema),
        "file-upload" => input::render_file_upload(schema),
        "search-input" => input::render_search_input(schema),
        "color-picker" => input::render_color_picker(schema),
        "slider" => input::render_slider(schema),
        // ── Action ────────────────────────────────────────────────────────
        "action-bar" => action::render_action_bar(schema),
        "dropdown-menu" => action::render_dropdown_menu(schema),
        "context-menu" => action::render_context_menu(schema),
        "fab" => action::render_fab(schema),
        "link" => action::render_link(schema),
        // ── Navigation ───────────────────────────────────────────────────
        "nav-bar" => navigation::render_nav_bar(schema),
        "sidebar" => navigation::render_sidebar(schema),
        "breadcrumb" => navigation::render_breadcrumb(schema),
        "pagination" => navigation::render_pagination(schema),
        "stepper" => navigation::render_stepper(schema),
        // ── Feedback ─────────────────────────────────────────────────────
        "alert" => feedback::render_alert(schema),
        "toast" => feedback::render_toast(schema),
        "modal" => feedback::render_modal(schema),
        "dialog" => feedback::render_dialog(schema),
        "loading-spinner" => feedback::render_loading_spinner(schema),
        "progress-bar" => feedback::render_progress_bar(schema),
        "empty-state" => feedback::render_empty_state(schema),
        "error-boundary" => feedback::render_error_boundary(schema),
        // ── Data-display ──────────────────────────────────────────────────
        "data-table" => data_display::render_data_table(schema),
        "badge" => data_display::render_badge(schema),
        "tag" => data_display::render_tag(schema),
        "avatar" => data_display::render_avatar(schema),
        "stat-card" => data_display::render_stat_card(schema),
        "timeline" => data_display::render_timeline(schema),
        "code-block" => data_display::render_code_block(schema),
        "json-viewer" => data_display::render_json_viewer(schema),
        "list" => data_display::render_list(schema),
        "detail-view" => data_display::render_detail_view(schema),
        // ── Layout ───────────────────────────────────────────────────────
        "container" => layout::render_container(schema),
        "row" => layout::render_row(schema),
        "column" => layout::render_column(schema),
        "grid" => layout::render_grid(schema),
        "stack" => layout::render_stack(schema),
        "divider" => layout::render_divider(schema),
        "spacer" => layout::render_spacer(schema),
        "scroll-area" => layout::render_scroll_area(schema),
        // ── Fallback ─────────────────────────────────────────────────────
        _ => render_generic(slug, schema, description),
    }
}

// ─── Fallback renderer ───────────────────────────────────────────────────────

pub(super) fn render_generic(slug: &str, schema: &Value, description: Option<&str>) -> String {
    let pretty = serde_json::to_string_pretty(schema).unwrap_or_default();
    let desc_html = match description {
        Some(desc) if !desc.is_empty() => format!(
            "<p class=\"text-base-content/70 mb-3\">{}</p>",
            html_escape(desc)
        ),
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
