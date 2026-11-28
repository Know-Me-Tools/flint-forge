use super::{json, Client, PgPool, Value};
use futures::{SinkExt, StreamExt};
use std::{net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
    MaybeTlsStream, WebSocketStream,
};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect(address: SocketAddr, payload: Value) -> Socket {
    let mut request = format!("ws://{address}/graphql")
        .into_client_request()
        .expect("websocket URL");
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "graphql-transport-ws".parse().expect("protocol"),
    );
    let (mut socket, response) = connect_async(request)
        .await
        .expect("upgrade GraphQL connection");
    assert_eq!(
        response.headers()["sec-websocket-protocol"],
        "graphql-transport-ws"
    );
    send(
        &mut socket,
        json!({"type":"connection_init","payload":payload}),
    )
    .await;
    socket
}

async fn send(socket: &mut Socket, payload: Value) {
    socket
        .send(Message::text(payload.to_string()))
        .await
        .expect("send operation");
}

async fn receive(socket: &mut Socket) -> Message {
    tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("timely GraphQL response")
        .expect("open connection")
        .expect("websocket message")
}

fn decode(message: &Message) -> Value {
    serde_json::from_str(message.to_text().expect("GraphQL frame")).expect("GraphQL JSON")
}

pub(super) async fn subscriptions(client: &Client, pool: &PgPool) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("gateway listener");
    let address = listener.local_addr().expect("loopback address");
    let app = client.app.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("serve gateway");
    });

    for (payload, reason) in [
        (json!({}), "missing authorization in connection_init"),
        (
            json!({"authorization":"expired"}),
            "invalid or expired token",
        ),
    ] {
        let mut socket = connect(address, payload).await;
        let Message::Close(Some(close)) = receive(&mut socket).await else {
            panic!("expected rejected init")
        };
        assert_eq!(close.reason.as_str(), reason);
    }

    let mut socket = connect(
        address,
        json!({"Authorization":format!("Bearer {}", client.token)}),
    )
    .await;
    assert_eq!(
        decode(&receive(&mut socket).await)["type"],
        "connection_ack"
    );
    send(
        &mut socket,
        json!({"type":"subscribe","id":"resume","payload":{
            "query":r#"subscription { noticesChanges(checkpoint:"old-cursor") { id } }"#
        }}),
    )
    .await;
    let error = decode(&receive(&mut socket).await);
    assert_eq!(error["id"], "resume");
    assert_eq!(
        error["payload"]["errors"][0]["message"], "invalid resume checkpoint",
        "{error}"
    );
    assert_eq!(decode(&receive(&mut socket).await)["type"], "complete");

    send(
        &mut socket,
        json!({"type":"subscribe","id":"live","payload":{
            "query":"subscription { noticesChanges { id body _op } }"
        }}),
    )
    .await;
    // The protocol has no per-subscription acknowledgement. Re-emit the same
    // change until the server has registered its receiver, with a bounded deadline.
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    let event = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            tokio::select! {
                frame = socket.next() => break decode(&frame.expect("open socket").expect("event")),
                _ = tick.tick() => {
                    sqlx::query("SELECT pg_notify('flint_change', $1)")
                        .bind(json!({"schema":"public","table":"notices","op":"update","record":{"id":"notice"}}).to_string())
                        .execute(pool).await.expect("publish notice");
                }
            }
        }
    }).await.expect("visible row delivered");
    assert_eq!(event["type"], "next");
    assert_eq!(event["id"], "live");
    assert_eq!(
        event["payload"]["data"]["noticesChanges"],
        json!({"id":"notice","body":"Message from the database","_op":"update"}),
        "{event}"
    );
    socket.close(None).await.expect("close connection");
    server.abort();
}
