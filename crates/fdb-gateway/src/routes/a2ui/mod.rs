//! A2UI Registry REST API routes.
//!
//! All routes under `/a2ui/v1/*` are protected by JWT authentication via the
//! `require_rls` middleware. They use a privileged `PgPool` from
//! `GatewayState` to call SECURITY DEFINER functions and read catalog data.
//!
//! # Endpoints
//!
//! - `GET    /a2ui/v1/components`
//! - `GET    /a2ui/v1/components/{slug}`
//! - `POST   /a2ui/v1/components/search`
//! - `GET    /a2ui/v1/components/bindings/{schema}/{table}`
//! - `GET    /a2ui/v1/applications`
//! - `GET    /a2ui/v1/applications/{id}`
//! - `GET    /a2ui/v1/catalog/{*catalog_id}`
//! - `POST   /a2ui/v1/surfaces/assemble`
#![forbid(unsafe_code)]

use crate::GatewayState;

mod applications;
mod catalog;
mod components;
mod helpers;
mod surfaces;

#[cfg(test)]
mod tests;

pub use applications::{get_application, get_design_system_tokens, list_applications};
pub use catalog::get_catalog;
pub use components::{
    get_bindings, get_component, get_component_value, list_components, list_components_value,
    search_components, search_components_value, ListComponentsQuery, SearchComponentsBody,
};
pub use surfaces::{assemble_surface, assemble_surface_value, AssembleSurfaceBody};

/// Route-scoped state for A2UI handlers. It intentionally exposes only the
/// privileged pool so tests and the production composition root can construct
/// it without building GraphQL/vector executors.
#[derive(Clone)]
pub struct A2uiState {
    pub pool: sqlx::PgPool,
}

impl From<GatewayState> for A2uiState {
    fn from(state: GatewayState) -> Self {
        Self { pool: state.pool }
    }
}
