//! Local server for the exact version of the Fabric WatchEntityType contract used by Quarry.
use fdb_domain::SubscriptionSpec;
use fdb_realtime::{FabricChangeSource, FrfConfig};
use forge_identity::RlsContext;
use frf_proto::fv2::{
    canonical_value::Kind,
    entity_service_server::{EntityService, EntityServiceServer},
    watch_entity_type_response::Frame,
    CanonicalValue, ChangeOp, ClientCheckpoint, EntityField, EntityKey, EntityMutation,
    EntityRecord, EntityType, KeyPart, WatchEntityTypeRequest, WatchEntityTypeResponse,
};
use futures::{stream, stream::BoxStream, StreamExt};
use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle};
use tonic::{
    transport::{server::TcpIncoming, Server},
    Code, Request, Response, Status,
};

pub fn spec() -> SubscriptionSpec {
    SubscriptionSpec {
        tenant: "tenant-a".into(),
        entity_type: "public.widgets".into(),
        filter: None,
        checkpoint: None,
    }
}

pub fn subscriber() -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: r#"{"tenant_id":"tenant-a","sub":"alice"}"#.into(),
        raw_bearer: "subscriber-token".into(),
        keto_subject: "alice".into(),
        vault_key_id: None,
    }
}

pub fn entity() -> EntityType {
    EntityType {
        schema: "public".into(),
        name: "widgets".into(),
        projection: "approved".into(),
    }
}

pub fn checkpoint() -> ClientCheckpoint {
    ClientCheckpoint {
        version: 1,
        token: vec![0, 190, 255],
        generation: 7,
    }
}

pub fn field(column: &str, kind: Kind) -> EntityField {
    EntityField {
        column: column.into(),
        value: Some(CanonicalValue { kind: Some(kind) }),
    }
}

pub fn mutation(op: ChangeOp) -> EntityMutation {
    let id = field("id", Kind::SignedInteger(4));
    EntityMutation {
        event_id: "event-7".into(),
        op: op.into(),
        entity_type: Some(entity()),
        tenant_id: spec().tenant,
        checkpoint: Some(checkpoint()),
        key: Some(EntityKey {
            parts: vec![KeyPart {
                column: id.column.clone(),
                value: id.value.clone(),
            }],
            canonical_id: "opaque-key".into(),
        }),
        record: (op != ChangeOp::Delete).then(|| EntityRecord { fields: vec![id] }),
        ..Default::default()
    }
}

#[derive(Clone)]
struct Service {
    received: mpsc::UnboundedSender<(String, WatchEntityTypeRequest)>,
    frames: Vec<Result<WatchEntityTypeResponse, Status>>,
    admission_error: Option<Code>,
}

#[tonic::async_trait]
impl EntityService for Service {
    type WatchEntityTypeStream = BoxStream<'static, Result<WatchEntityTypeResponse, Status>>;

    async fn watch_entity_type(
        &self,
        request: Request<WatchEntityTypeRequest>,
    ) -> Result<Response<Self::WatchEntityTypeStream>, Status> {
        let auth = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        self.received
            .send((auth, request.into_inner()))
            .map_err(|_| Status::cancelled("test finished"))?;
        if let Some(code) = self.admission_error {
            return Err(Status::new(code, "admission"));
        }
        Ok(Response::new(stream::iter(self.frames.clone()).boxed()))
    }
}

pub struct Fixture {
    pub source: FabricChangeSource,
    pub received: mpsc::UnboundedReceiver<(String, WatchEntityTypeRequest)>,
    server: JoinHandle<Result<(), tonic::transport::Error>>,
}

impl Fixture {
    pub async fn new(frames: Vec<Result<Frame, Status>>, admission_error: Option<Code>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("loopback listener");
        let endpoint = format!("http://{}", listener.local_addr().expect("address"));
        let (sent, received) = mpsc::unbounded_channel();
        let frames = frames
            .into_iter()
            .map(|r| r.map(|frame| WatchEntityTypeResponse { frame: Some(frame) }))
            .collect();
        let service = Service {
            received: sent,
            frames,
            admission_error,
        };
        let server = tokio::spawn(
            Server::builder()
                .add_service(EntityServiceServer::new(service))
                .serve_with_incoming(TcpIncoming::from(listener)),
        );
        let source = FabricChangeSource::new(
            FrfConfig {
                endpoint,
                projection: "approved".into(),
            },
            None,
        )
        .expect("fabric source");
        Self {
            source,
            received,
            server,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
