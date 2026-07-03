// This example crate is almost entirely wit-bindgen-generated bindings. The
// `bindings::export!` macro expands generated `__export_*` items into this crate's
// scope, tripping clippy::pedantic's `used_underscore_items`; a module- or
// statement-scoped allow does not reach the macro expansion, so allow it crate-wide.
// Scope is the example crate only — the `crates/*` library lint posture is unaffected.
#![allow(clippy::used_underscore_items)]

#[allow(warnings)]
mod bindings;

use bindings::exports::wasi::http::incoming_handler::Guest;
use bindings::wasi::http::types::{Headers, OutgoingBody, OutgoingResponse, ResponseOutparam};

struct HelloComponent;

impl Guest for HelloComponent {
    fn handle(
        _request: bindings::wasi::http::types::IncomingRequest,
        response_out: ResponseOutparam,
    ) {
        let headers = Headers::new();
        let response = OutgoingResponse::new(headers);
        response.set_status_code(200).unwrap();

        let body = response.body().unwrap();
        let out = body.write().unwrap();
        out.blocking_write_and_flush(b"Hello from Flint edge function!\n")
            .unwrap();
        drop(out);
        OutgoingBody::finish(body, None).unwrap();

        ResponseOutparam::set(response_out, Ok(response));
    }
}

bindings::export!(HelloComponent with_types_in bindings);
