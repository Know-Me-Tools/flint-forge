//! GraphQL query/mutation delegation and introspection support.
//!
//! Query and Mutation execution is delegated directly to `graphql.resolve()`
//! inside Postgres under RLS (see crate-level docs); this module currently
//! hosts the introspection merger that stitches the pg_graphql schema
//! together with the sibling subscription SDL.
pub mod introspection;
