//! The compilers that turn a validated `DatabaseModel` into servable
//! artifacts: an Axum REST router (`rest`), an OpenAPI document
//! (`openapi`), a GraphQL subscription schema (`graphql`), MCP tool
//! definitions (`mcp`), and A2UI surface assembly (`a2ui`). `filters`
//! and `embed_schema` are shared helpers used by more than one compiler.

pub mod a2ui;
pub mod embed_schema;
pub mod filters;
pub mod graphql;
pub mod mcp;
/// Compiles a `DatabaseModel` into an OpenAPI 3.1.0 JSON document, served at
/// `GET /openapi.json`.
pub mod openapi;
/// Compiles a `DatabaseModel` into an Axum `Router` exposing PostgREST-style
/// CRUD + `/rpc` endpoints under RLS.
pub mod rest;
