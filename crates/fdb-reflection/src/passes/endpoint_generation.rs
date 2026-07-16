use crate::model::{DatabaseModel, FnMeta, Table};

/// Describes one HTTP endpoint derived from the `DatabaseModel`.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub method: &'static str,
    pub path: String,
    pub kind: EndpointKind,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum EndpointKind {
    TableList { table: Table },
    TableById { table: Table },
    Rpc { func: FnMeta },
}

/// Produce the ordered list of endpoints from a validated `DatabaseModel`.
/// Consumed by `RestCompiler::compile()`.
pub fn generate(model: &DatabaseModel) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();

    for table in &model.tables {
        let prefix = format!("/{}/{}", table.schema, table.name);
        endpoints.push(Endpoint {
            method: "GET",
            path: prefix.clone(),
            kind: EndpointKind::TableList {
                table: table.clone(),
            },
        });
        endpoints.push(Endpoint {
            method: "POST",
            path: prefix.clone(),
            kind: EndpointKind::TableList {
                table: table.clone(),
            },
        });
        endpoints.push(Endpoint {
            method: "PATCH",
            path: prefix.clone(),
            kind: EndpointKind::TableById {
                table: table.clone(),
            },
        });
        endpoints.push(Endpoint {
            method: "DELETE",
            path: prefix.clone(),
            kind: EndpointKind::TableById {
                table: table.clone(),
            },
        });
    }

    // Postgres allows function overloading (same schema+name, different
    // argument lists) — `cron.schedule(text, text)` and
    // `cron.schedule(text, text, text)` both exist. REST exposes one path per
    // function *name*, not per overload (matching PostgREST semantics): a
    // second overload must not register a second route for the same path, or
    // the router panics on the duplicate. `rpc::handle_rpc` resolves the
    // correct overload at call time from the request body's keys.
    let mut seen_rpc_paths = std::collections::HashSet::new();
    for func in &model.functions {
        let path = format!("/rpc/{}/{}", func.schema, func.name);
        if !seen_rpc_paths.insert(path.clone()) {
            continue;
        }
        endpoints.push(Endpoint {
            method: "POST",
            path,
            kind: EndpointKind::Rpc { func: func.clone() },
        });
    }

    endpoints
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ArgMeta;

    fn empty_model(functions: Vec<FnMeta>) -> DatabaseModel {
        DatabaseModel {
            tables: vec![],
            functions,
            views: vec![],
            version: 1,
        }
    }

    fn overload(args: Vec<ArgMeta>) -> FnMeta {
        FnMeta {
            schema: "cron".into(),
            name: "schedule".into(),
            args,
            return_type: "bigint".into(),
            security_definer: false,
        }
    }

    /// p16-c-followup gate: Postgres allows function overloading (e.g.
    /// `cron.schedule(text, text)` and `cron.schedule(text, text, text)`).
    /// `generate()` must emit exactly one `/rpc/<schema>/<name>` route per
    /// distinct path — a second overload must never register a duplicate
    /// route, which previously panicked the Axum router at startup.
    #[test]
    fn generate_dedupes_overloaded_rpc_functions_by_path() {
        let two_arg = overload(vec![
            ArgMeta {
                name: "schedule".into(),
                pg_type: "text".into(),
            },
            ArgMeta {
                name: "command".into(),
                pg_type: "text".into(),
            },
        ]);
        let three_arg = overload(vec![
            ArgMeta {
                name: "job_name".into(),
                pg_type: "text".into(),
            },
            ArgMeta {
                name: "schedule".into(),
                pg_type: "text".into(),
            },
            ArgMeta {
                name: "command".into(),
                pg_type: "text".into(),
            },
        ]);
        let model = empty_model(vec![two_arg, three_arg]);

        let endpoints = generate(&model);

        let rpc_paths: Vec<&str> = endpoints
            .iter()
            .filter(|e| matches!(e.kind, EndpointKind::Rpc { .. }))
            .map(|e| e.path.as_str())
            .collect();
        assert_eq!(
            rpc_paths,
            vec!["/rpc/cron/schedule"],
            "two overloads of the same function must yield exactly one route"
        );
    }

    #[test]
    fn generate_emits_distinct_paths_for_distinct_functions() {
        let a = FnMeta {
            schema: "public".into(),
            name: "calculate_total".into(),
            args: vec![],
            return_type: "numeric".into(),
            security_definer: false,
        };
        let b = FnMeta {
            schema: "public".into(),
            name: "calculate_tax".into(),
            args: vec![],
            return_type: "numeric".into(),
            security_definer: false,
        };
        let model = empty_model(vec![a, b]);

        let endpoints = generate(&model);
        let rpc_paths: Vec<&str> = endpoints
            .iter()
            .filter(|e| matches!(e.kind, EndpointKind::Rpc { .. }))
            .map(|e| e.path.as_str())
            .collect();
        assert_eq!(
            rpc_paths,
            vec!["/rpc/public/calculate_total", "/rpc/public/calculate_tax"]
        );
    }
}
