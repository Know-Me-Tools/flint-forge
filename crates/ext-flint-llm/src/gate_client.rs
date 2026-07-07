//! HTTP client for the flint-gate/UAR LLM bridge.
//!
//! This module is intentionally free of pgrx dependencies so it can run on a
//! dedicated tokio runtime thread without touching Postgres internals.

use crate::error::{LlmError, Result};
use reqwest::header::{self, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

const DEFAULT_EMBED_MODEL: &str = "text-embedding-3-small";
const DEFAULT_COMPLETE_MODEL: &str = "gpt-4.1-nano";

/// Default flint-gate base URL, overridable via the `FLINT_GATE_URL` environment variable.
pub fn default_base_url() -> String {
    std::env::var("FLINT_GATE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Request body for the flint-gate embedding endpoint.
#[derive(Debug, Serialize)]
pub struct EmbedRequest {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Response body from the flint-gate embedding endpoint.
#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    pub embedding: Vec<f32>,
    #[serde(default)]
    pub model: String,
}

/// Request body for the flint-gate completion endpoint.
#[derive(Debug, Serialize)]
pub struct CompleteRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

/// Response body from the flint-gate completion endpoint.
#[derive(Debug, Deserialize)]
pub struct CompleteResponse {
    pub content: String,
    #[serde(default)]
    pub model: String,
}

/// Authenticated client that talks to flint-gate on behalf of an origin identity.
pub struct GateClient {
    client: reqwest::Client,
    base_url: String,
    service_token: SecretString,
}

impl GateClient {
    /// Build a new client.
    ///
    /// `base_url` may or may not include a trailing slash.
    pub fn new(base_url: String, service_token: SecretString) -> Result<Self> {
        let base_url = base_url.trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(LlmError::Config("flint-gate base URL is empty".to_string()));
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            base_url,
            service_token,
        })
    }

    /// Resolve a model name, mapping `"default"` to the embedding default.
    fn resolve_embed_model(model: Option<&str>) -> String {
        match model {
            None | Some("default") | Some("") => DEFAULT_EMBED_MODEL.to_string(),
            Some(m) => m.to_string(),
        }
    }

    /// Resolve a model name, mapping `"default"` to the completion default.
    fn resolve_complete_model(model: Option<&str>) -> String {
        match model {
            None | Some("default") | Some("") => DEFAULT_COMPLETE_MODEL.to_string(),
            Some(m) => m.to_string(),
        }
    }

    /// Common headers for every request.
    fn auth_headers(&self, origin_jwt: Option<&str>) -> Result<HeaderMap> {
        let bearer = format!("Bearer {}", self.service_token.expose_secret());
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&bearer)
                .map_err(|e| LlmError::Config(format!("bad auth header: {e}")))?,
        );
        if let Some(origin) = origin_jwt {
            headers.insert(
                "X-Forge-Origin-JWT",
                HeaderValue::from_str(origin)
                    .map_err(|e| LlmError::Config(format!("bad origin JWT: {e}")))?,
            );
        }
        Ok(headers)
    }

    /// Request an embedding from flint-gate.
    ///
    /// `model` may be `None` or `"default"` to use the configured default.
    pub async fn embed(
        &self,
        input: &str,
        model: Option<&str>,
        origin_jwt: Option<&str>,
    ) -> Result<Vec<f32>> {
        let req = EmbedRequest {
            input: input.to_string(),
            model: Some(Self::resolve_embed_model(model)),
        };
        let url = format!("{}/v1/llm/embed", self.base_url);
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers(origin_jwt)?)
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            let body: EmbedResponse = resp
                .json()
                .await
                .map_err(|e| LlmError::BadResponse(format!("embed response malformed: {e}")))?;
            Ok(body.embedding)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(LlmError::from_response(status.as_u16(), body))
        }
    }

    /// Request a completion from flint-gate.
    ///
    /// `model` may be `None` or `"default"` to use the configured default.
    /// `options` is forwarded to the gateway as-is under the `options` key.
    pub async fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        options: Option<&serde_json::Value>,
        origin_jwt: Option<&str>,
    ) -> Result<String> {
        let req = CompleteRequest {
            prompt: prompt.to_string(),
            model: Some(Self::resolve_complete_model(model)),
            options: options.cloned(),
        };
        let url = format!("{}/v1/llm/complete", self.base_url);
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers(origin_jwt)?)
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            let body: CompleteResponse = resp
                .json()
                .await
                .map_err(|e| LlmError::BadResponse(format!("complete response malformed: {e}")))?;
            Ok(body.content)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(LlmError::from_response(status.as_u16(), body))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_request_serializes() {
        let req = EmbedRequest {
            input: "hello".to_string(),
            model: Some("text-embedding-3-small".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("text-embedding-3-small"));
    }

    #[test]
    fn embed_response_deserializes() {
        let raw = r#"{"embedding":[0.1,0.2,0.3],"model":"m"}"#;
        let resp: EmbedResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.embedding.len(), 3);
        assert_eq!(resp.model, "m");
    }

    #[test]
    fn complete_response_deserializes() {
        let raw = r#"{"content":"hi","model":"m"}"#;
        let resp: CompleteResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.content, "hi");
    }

    #[test]
    fn default_model_resolution() {
        assert_eq!(
            GateClient::resolve_embed_model(None),
            "text-embedding-3-small"
        );
        assert_eq!(
            GateClient::resolve_embed_model(Some("default")),
            "text-embedding-3-small"
        );
        assert_eq!(GateClient::resolve_embed_model(Some("custom")), "custom");
    }
}
