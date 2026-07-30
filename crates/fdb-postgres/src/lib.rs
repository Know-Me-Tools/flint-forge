//! Postgres adapters: DatabaseBackend, SchemaProvider, RestExecutor, GraphQlExecutor (pg_graphql), pgvector.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod backend;
pub mod conn;
pub mod error;
mod graphql;
mod provisioner;
mod rest;
mod vector_rpc;

pub use backend::PgBackend;
pub use graphql::PgGraphQl;
pub use provisioner::PgProvisioner;
pub use rest::PgRest;
pub use vector_rpc::PgVectorRpc;
