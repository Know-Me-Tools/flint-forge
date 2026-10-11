//! Fabric `WatchEntityType` adapter.

mod map;

use std::sync::Arc;

use async_trait::async_trait;
use fdb_domain::{ChangeEvent, SubscriptionSpec};
use fdb_ports::{ChangeStreamSource, StreamError};
use forge_identity::RlsContext;
use frf_proto::fv2::{
    entity_service_client::EntityServiceClient, watch_entity_type_request, EntityType, LiveStart,
    WatchEntityTypeRequest,
};
use futures::{stream::BoxStream, StreamExt as _};
use tonic::{metadata::MetadataValue, transport::Channel, Code, Request};
use tracing::instrument;

use crate::{keto_check_via_http, KetoConfig};
use map::{decode_checkpoint, map_response};

/// Configuration for the FRF gRPC endpoint and approved projection.
#[derive(Debug, Clone)]
pub struct FrfConfig {
    /// Full gRPC address, e.g. `http://frf:9090`.
    pub endpoint: String,
    /// Fabric enrollment projection name. Defaults to `default` in gateway
    /// composition and cannot be supplied by a GraphQL subscriber.
    pub projection: String,
}

/// `ChangeStreamSource` backed by Fabric's authorized v2 entity-type stream.
#[derive(Clone)]
pub struct FabricChangeSource {
    channel: Arc<Channel>,
    http: reqwest::Client,
    keto: Option<KetoConfig>,
    projection: String,
}

impl FabricChangeSource {
    /// Build a lazy, connection-pooled Fabric client.
    ///
    /// # Errors
    ///
    /// Returns [`FabricError::Connect`] for an invalid endpoint and
    /// [`FabricError::Configuration`] for an invalid projection identifier.
    pub fn new(frf: FrfConfig, keto: Option<KetoConfig>) -> Result<Self, FabricError> {
        if !safe_identifier(&frf.projection) {
            return Err(FabricError::Configuration("invalid projection"));
        }
        let channel = Channel::from_shared(frf.endpoint)
            .map_err(|error| FabricError::Connect(error.to_string()))?
            .connect_lazy();
        Ok(Self {
            channel: Arc::new(channel),
            http: reqwest::Client::new(),
            keto,
            projection: frf.projection,
        })
    }

    async fn keto_check(
        &self,
        entity_type: &str,
        tenant: &str,
        subject: &str,
    ) -> Result<(), StreamError> {
        let Some(keto) = &self.keto else {
            return Ok(());
        };
        keto_check_via_http(&self.http, &keto.base_url, entity_type, tenant, subject).await
    }

    async fn open(
        &self,
        spec: &SubscriptionSpec,
        who: &RlsContext,
    ) -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError> {
        let (schema, name) = split_entity_type(&spec.entity_type)?;
        if !safe_tenant(&spec.tenant) {
            return Err(StreamError::Denied);
        }

        let start = match spec.checkpoint.as_deref() {
            Some(encoded) => watch_entity_type_request::Start::Resume(decode_checkpoint(encoded)?),
            None => watch_entity_type_request::Start::Live(LiveStart {}),
        };
        let request_body = WatchEntityTypeRequest {
            entity_type: Some(EntityType {
                schema: schema.to_owned(),
                name: name.to_owned(),
                projection: self.projection.clone(),
            }),
            tenant_id: spec.tenant.clone(),
            start: Some(start),
        };
        let authorization = MetadataValue::try_from(format!("Bearer {}", who.raw_bearer))
            .map_err(|_| StreamError::Denied)?;
        let mut request = Request::new(request_body);
        request
            .metadata_mut()
            .insert("authorization", authorization);

        let mut client = EntityServiceClient::new(self.channel.as_ref().clone());
        let response = client
            .watch_entity_type(request)
            .await
            .map_err(status_to_stream_error)?;
        let expected = map::ExpectedScope {
            schema: schema.to_owned(),
            table: name.to_owned(),
            projection: self.projection.clone(),
            tenant: spec.tenant.clone(),
        };
        Ok(response
            .into_inner()
            .filter_map(move |message| {
                let expected = expected.clone();
                async move {
                    match message {
                        Ok(response) => map_response(response, &expected).transpose(),
                        Err(status) => Some(Err(status_to_stream_error(status))),
                    }
                }
            })
            .boxed())
    }
}

#[async_trait]
impl ChangeStreamSource for FabricChangeSource {
    #[instrument(skip(self, who), fields(entity_type = %spec.entity_type), err)]
    async fn watch(
        &self,
        spec: SubscriptionSpec,
        who: &RlsContext,
    ) -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError> {
        split_entity_type(&spec.entity_type)?;
        if !safe_tenant(&spec.tenant) {
            return Err(StreamError::Denied);
        }
        self.keto_check(&spec.entity_type, &spec.tenant, &who.keto_subject)
            .await?;
        self.open(&spec, who).await
    }
}

/// Errors from Fabric adapter construction.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FabricError {
    /// The tonic channel rejected the endpoint URI.
    #[error("gRPC connect: {0}")]
    Connect(String),
    /// A static adapter setting was invalid.
    #[error("configuration: {0}")]
    Configuration(&'static str),
}

fn status_to_stream_error(status: tonic::Status) -> StreamError {
    match status.code() {
        Code::PermissionDenied | Code::Unauthenticated => StreamError::Denied,
        Code::FailedPrecondition | Code::OutOfRange => StreamError::ResnapshotRequired,
        Code::ResourceExhausted => StreamError::Lagged,
        _ => StreamError::Unavailable,
    }
}

fn split_entity_type(entity_type: &str) -> Result<(&str, &str), StreamError> {
    let Some((schema, name)) = entity_type.split_once('.') else {
        return Err(StreamError::Denied);
    };
    if name.contains('.') || !safe_identifier(schema) || !safe_identifier(name) {
        return Err(StreamError::Denied);
    }
    Ok((schema, name))
}

fn safe_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
        && value.len() <= 63
}

fn safe_tenant(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}
