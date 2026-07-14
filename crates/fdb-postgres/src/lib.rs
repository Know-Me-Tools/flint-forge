//! Postgres adapters: DatabaseBackend, SchemaProvider, RestExecutor, GraphQlExecutor (pg_graphql), pgvector.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod conn;
pub mod error;
mod backend;
mod graphql;
mod rest;
mod vector_rpc;

pub use backend::PgBackend;
pub use graphql::PgGraphQl;
pub use rest::PgRest;
pub use vector_rpc::PgVectorRpc;
