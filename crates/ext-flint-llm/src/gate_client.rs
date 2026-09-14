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
    openai: bool,
    embedding_dimensions: Option<usize>,
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
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        let openai = match std::env::var("FLINT_LLM_PROTOCOL").as_deref() {
            Ok("openai") => true,
            Ok("legacy") | Err(_) => false,
            Ok(_) => {
                return Err(LlmError::Config(
                    "FLINT_LLM_PROTOCOL must be openai or legacy".into(),
                ))
            }
        };
        let embedding_dimensions = std::env::var("FLINT_LLM_EMBED_DIMENSIONS")
            .ok()
            .map(|value| {
                value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| {
                        LlmError::Config("FLINT_LLM_EMBED_DIMENSIONS must be positive".into())
                    })
            })
            .transpose()?;
        Ok(Self {
            embedding_dimensions,
            client,
            base_url,
            service_token,
            openai,
        })
    }

    /// Resolve a model name, mapping `"default"` to the embedding default.
    fn resolve_embed_model(model: Option<&str>) -> String {
        match model {
            None | Some("default") | Some("") => std::env::var("FLINT_LLM_EMBED_MODEL")
                .unwrap_or_else(|_| DEFAULT_EMBED_MODEL.to_string()),
            Some(m) => m.to_string(),
        }
    }

    /// Resolve a model name, mapping `"default"` to the completion default.
    fn resolve_complete_model(model: Option<&str>) -> String {
        match model {
            None | Some("default") | Some("") => std::env::var("FLINT_LLM_CHAT_MODEL")
                .unwrap_or_else(|_| DEFAULT_COMPLETE_MODEL.to_string()),
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
        if self.openai {
            headers.insert(
                "X-API-Key",
                HeaderValue::from_str(self.service_token.expose_secret())
                    .map_err(|_| LlmError::Config("invalid service token header".into()))?,
            );
        }
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
        let path = if self.openai {
            "/v1/embeddings"
        } else {
            "/v1/llm/embed"
        };
        let url = format!("{}{path}", self.base_url);
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers(origin_jwt)?)
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            let embedding = if self.openai {
                let body: OpenAiEmbeddings = resp.json().await.map_err(|_| {
                    LlmError::BadResponse("malformed OpenAI embedding response".into())
                })?;
                if body.data.len() != 1 || body.data[0].index != 0 {
                    return Err(LlmError::BadResponse(
                        "expected one embedding at index zero".into(),
                    ));
                }
                body.data
                    .into_iter()
                    .next()
                    .map(|v| v.embedding)
                    .unwrap_or_default()
            } else {
                resp.json::<EmbedResponse>().await?.embedding
            };
            if embedding.is_empty() || embedding.iter().any(|v| !v.is_finite()) {
                return Err(LlmError::BadResponse(
                    "empty or non-finite embedding".into(),
                ));
            }
            if self
                .embedding_dimensions
                .is_some_and(|expected| embedding.len() != expected)
            {
                return Err(LlmError::BadResponse("embedding dimension mismatch".into()));
            }
            Ok(embedding)
        } else {
            Err(LlmError::from_response(
                status.as_u16(),
                "model gateway rejected request".into(),
            ))
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
        let path = if self.openai {
            "/v1/chat/completions"
        } else {
            "/v1/llm/complete"
        };
        let url = format!("{}{path}", self.base_url);
        let body = if self.openai {
            let mut body = match options {
                Some(serde_json::Value::Object(map)) => map.clone(),
                None => serde_json::Map::new(),
                _ => {
                    return Err(LlmError::Config(
                        "completion options must be an object".into(),
                    ))
                }
            };
            body.insert("model".into(), serde_json::json!(req.model));
            body.insert(
                "messages".into(),
                serde_json::json!([{"role":"user", "content":prompt}]),
            );
            body.insert("stream".into(), serde_json::json!(false));
            serde_json::Value::Object(body)
        } else {
            serde_json::to_value(&req)
                .map_err(|_| LlmError::Config("invalid completion request".into()))?
        };
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers(origin_jwt)?)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            if self.openai {
                let body: OpenAiCompletion = resp.json().await.map_err(|_| {
                    LlmError::BadResponse("malformed OpenAI completion response".into())
                })?;
                body.choices
                    .into_iter()
                    .next()
                    .and_then(|c| c.message.content)
                    .ok_or_else(|| LlmError::BadResponse("missing completion content".into()))
            } else {
                Ok(resp.json::<CompleteResponse>().await?.content)
            }
        } else {
            Err(LlmError::from_response(
                status.as_u16(),
                "model gateway rejected request".into(),
            ))
        }
    }
}

#[derive(Deserialize)]
struct OpenAiEmbeddings {
    data: Vec<OpenAiEmbedding>,
}
#[derive(Deserialize)]
struct OpenAiEmbedding {
    index: usize,
    embedding: Vec<f32>,
}
#[derive(Deserialize)]
struct OpenAiCompletion {
    choices: Vec<OpenAiChoice>,
}
#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}
#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
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
