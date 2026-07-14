//! Feedback renderers: alert, toast, modal, dialog, spinner, progress, empty/error states.
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_collect)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::bool_assert_comparison)]

use serde_json::Value;

pub(super) fn render_alert(schema: &Value) -> String {
    let msg = schema
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Alert message");
    let variant = schema
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or("info");
    let cls = match variant {
        "success" => "alert-success",
        "warning" => "alert-warning",
        "error" => "alert-error",
        _ => "alert-info",
    };
    format!(
        r#"<div data-flint-component="alert" class="alert {cls}">
  <span>{msg}</span>
</div>"#
    )
}

pub(super) fn render_toast(schema: &Value) -> String {
    let msg = schema
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Toast notification");
    let variant = schema
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or("success");
    let cls = match variant {
        "error" => "alert-error",
        "warning" => "alert-warning",
        "info" => "alert-info",
        _ => "alert-success",
    };
    format!(
        r#"<div data-flint-component="toast" class="toast toast-end">
  <div class="alert {cls}"><span>{msg}</span></div>
</div>"#
    )
}

pub(super) fn render_modal(schema: &Value) -> String {
    let title = schema
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Modal");
    let size = match schema.get("size").and_then(Value::as_str).unwrap_or("md") {
        "sm" => "max-w-sm",
        "lg" => "max-w-2xl",
        "xl" => "max-w-4xl",
        _ => "max-w-lg",
    };
    format!(
        r#"<div data-flint-component="modal" class="mockup-window border border-base-300 {size}">
  <div class="p-4 bg-base-100">
    <h3 class="font-bold text-lg mb-2">{title}</h3>
    <p class="py-4 text-base-content/70">Modal content goes here.</p>
    <div class="modal-action"><button class="btn btn-ghost btn-sm">Close</button><button class="btn btn-primary btn-sm">Confirm</button></div>
  </div>
</div>"#
    )
}

pub(super) fn render_dialog(schema: &Value) -> String {
    let title = schema
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Are you sure?");
    let message = schema
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("This action cannot be undone.");
    let confirm = schema
        .get("confirm_label")
        .and_then(Value::as_str)
        .unwrap_or("Confirm");
    let cancel = schema
        .get("cancel_label")
        .and_then(Value::as_str)
        .unwrap_or("Cancel");
    format!(
        r#"<div data-flint-component="dialog" class="card bg-base-100 shadow w-96">
  <div class="card-body">
    <h2 class="card-title">{title}</h2>
    <p class="text-base-content/70">{message}</p>
    <div class="card-actions justify-end mt-4">
      <button class="btn btn-ghost btn-sm">{cancel}</button>
      <button class="btn btn-error btn-sm">{confirm}</button>
    </div>
  </div>
</div>"#
    )
}

pub(super) fn render_loading_spinner(schema: &Value) -> String {
    let size = match schema.get("size").and_then(Value::as_str).unwrap_or("md") {
        "sm" => "loading-sm",
        "lg" => "loading-lg",
        "xl" => "loading-xl",
        _ => "loading-md",
    };
    let label = schema.get("label").and_then(Value::as_str);
    let label_html = label
        .map(|l| format!(r#"<span class="text-sm text-base-content/60">{l}</span>"#))
        .unwrap_or_default();
    format!(
        r#"<div data-flint-component="loading-spinner" class="flex items-center gap-3">
  <span class="loading loading-spinner {size}"></span>
  {label_html}
</div>"#
    )
}

pub(super) fn render_progress_bar(schema: &Value) -> String {
    let value = schema.get("value").and_then(Value::as_i64).unwrap_or(60);
    let max = schema.get("max").and_then(Value::as_i64).unwrap_or(100);
    let indeterminate = schema
        .get("indeterminate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let val_attr = if indeterminate {
        String::new()
    } else {
        format!("value=\"{value}\" max=\"{max}\"")
    };
    format!(
        r#"<div data-flint-component="progress-bar" class="w-full">
  <progress class="progress progress-primary w-full" {val_attr}></progress>
  {}
</div>"#,
        if indeterminate {
            String::new()
        } else {
            format!("<p class=\"text-sm text-right text-base-content/50\">{value}%</p>")
        }
    )
}

pub(super) fn render_empty_state(schema: &Value) -> String {
    let title = schema
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("No items found");
    let desc = schema
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let action = schema
        .get("action")
        .and_then(|a| a.get("label"))
        .and_then(Value::as_str);
    let btn = action
        .map(|l| format!(r#"<button class="btn btn-primary btn-sm mt-3">{l}</button>"#))
        .unwrap_or_default();
    format!(
        r#"<div data-flint-component="empty-state" class="flex flex-col items-center justify-center p-12 text-center">
  <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 text-base-content/20 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"/></svg>
  <h3 class="text-lg font-medium">{title}</h3>
  <p class="text-base-content/50 text-sm mt-1">{desc}</p>
  {btn}
</div>"#
    )
}

pub(super) fn render_error_boundary(schema: &Value) -> String {
    let msg = schema
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Something went wrong.");
    let retry = schema
        .get("retry_label")
        .and_then(Value::as_str)
        .unwrap_or("Try again");
    format!(
        r#"<div data-flint-component="error-boundary" class="alert alert-error shadow-lg">
  <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 shrink-0 stroke-current" fill="none" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
  <span>{msg}</span>
  <button class="btn btn-sm btn-ghost">{retry}</button>
</div>"#
    )
}
