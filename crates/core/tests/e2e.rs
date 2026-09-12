use std::time::Duration;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use quiz_buzzer_core::{bind_server, ServerConfig};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

const HOST_KEY: &str = "0123456789abcdef0123456789abcdef";

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Client {
    sink: SplitSink<WsStream, Message>,
    stream: SplitStream<WsStream>,
}

impl Client {
    async fn connect(port: u16) -> Self {
        let url = format!("ws://127.0.0.1:{port}/ws");
        let (ws, _) = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match connect_async(&url).await {
                    Ok(ok) => return ok,
                    Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
                }
            }
        })
        .await
        .expect("timed out connecting to websocket");
        let (sink, stream) = ws.split();
        Self { sink, stream }
    }

    async fn send(&mut self, v: Value) {
        self.sink
            .send(Message::text(v.to_string()))
            .await
            .expect("send websocket text");
    }

    async fn next_json(&mut self) -> Value {
        loop {
            let msg = self
                .stream
                .next()
                .await
                .expect("websocket closed")
                .expect("websocket error");
            match msg {
                Message::Text(text) => {
                    return serde_json::from_str(text.as_str()).expect("invalid json from server");
                }
                Message::Ping(_) | Message::Pong(_) => {}
                other => panic!("unexpected websocket frame: {other:?}"),
            }
        }
    }

    async fn wait(&mut self, pred: impl Fn(&Value) -> bool) -> Value {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let v = self.next_json().await;
                if pred(&v) {
                    return v;
                }
            }
        })
        .await
        .expect("timed out waiting for websocket message")
    }

    async fn wait_hello_ok(&mut self) -> Value {
        self.wait(|v| v.get("type").and_then(Value::as_str) == Some("helloOk"))
            .await
    }

    async fn wait_snapshot(&mut self, pred: impl Fn(&Value) -> bool) -> Value {
        self.wait(|v| v.get("type").and_then(Value::as_str) == Some("snapshot") && pred(v))
            .await
    }
}

fn sequence(snap: &Value) -> &[Value] {
    snap.get("sequence")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

#[tokio::test]
async fn websocket_lockout_order_b_then_a() {
    let static_dir =
        std::env::temp_dir().join(format!("quiz-buzzer-e2e-static-{}", std::process::id()));
    std::fs::create_dir_all(&static_dir).unwrap();

    let (listener, app) = bind_server(ServerConfig {
        port: 0,
        host_key: HOST_KEY.into(),
        static_dir,
    })
    .await
    .expect("bind_server");
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut clicker = Client::connect(port).await;
    let mut player_a = Client::connect(port).await;
    let mut player_b = Client::connect(port).await;

    clicker
        .send(json!({"type": "hello", "role": "clicker", "hostKey": HOST_KEY}))
        .await;
    clicker.wait_hello_ok().await;

    player_a
        .send(json!({"type": "hello", "role": "player", "name": "A", "playerId": null}))
        .await;
    player_a.wait_hello_ok().await;

    player_b
        .send(json!({"type": "hello", "role": "player", "name": "B", "playerId": null}))
        .await;
    player_b.wait_hello_ok().await;

    clicker.send(json!({"type": "arm"})).await;
    clicker
        .wait_snapshot(|s| s.get("accepting") == Some(&Value::Bool(true)))
        .await;

    player_b.send(json!({"type": "buzz"})).await;
    player_a.send(json!({"type": "buzz"})).await;

    let locked = clicker.wait_snapshot(|s| sequence(s).len() == 2).await;
    let seq = sequence(&locked);
    assert_eq!(seq[0]["name"], "B");
    assert_eq!(seq[0]["place"], 1);
    assert!(seq[0].get("playerId").is_some());
    assert_eq!(seq[1]["name"], "A");
    assert_eq!(seq[1]["place"], 2);

    clicker.send(json!({"type": "reset"})).await;
    let reset = clicker
        .wait_snapshot(|s| {
            s.get("accepting") == Some(&Value::Bool(false)) && sequence(s).len() == 2
        })
        .await;
    assert_eq!(reset["accepting"], false);
    assert_eq!(sequence(&reset).len(), 2);

    clicker.send(json!({"type": "arm"})).await;
    let armed = clicker.wait_snapshot(|s| sequence(s).is_empty()).await;
    assert!(sequence(&armed).is_empty());
    assert_eq!(armed["accepting"], true);

    server.abort();
}
