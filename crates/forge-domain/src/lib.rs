//! Forge core domain — pure cross-cutting types. No infrastructure dependencies.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Tenant identifier (newtype over UUID-as-string for transport stability).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct TenantId(pub String);

/// Authenticated subject identifier (the JWT `sub`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SubjectId(pub String);

/// JSON value alias used across ports.
pub type Json = serde_json::Value;

/// Top-level error surfaced across subsystem boundaries.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ForgeError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("backend: {0}")]
    Backend(String),
    #[error("policy denied")]
    PolicyDenied,
}
