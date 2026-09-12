use std::path::PathBuf;
use std::time::Duration;

use axum::extract::ws::{close_code, CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, oneshot};
use tower_http::services::{ServeDir, ServeFile};

use crate::actor::{Command, RoomHandle};
use crate::handler::{apply_client_message, Conn};
use crate::protocol::{ClientMessage, ClientRole, ServerMessage, You};
use crate::room::Snapshot;

const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TEXT_BYTES: usize = 4096;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub host_key: String,
    pub static_dir: PathBuf,
}

pub fn router(handle: RoomHandle, static_dir: PathBuf) -> Router {
    let player = ServeFile::new(static_dir.join("player.html"));
    let board = ServeFile::new(static_dir.join("board.html"));
    let host = ServeFile::new(static_dir.join("host.html"));
    let ws = Router::new()
        .route("/ws", get(upgrade_ws))
        .with_state(handle);
    Router::new()
        .merge(ws)
        .route_service("/", player)
        .route_service("/board", board)
        .route_service("/host", host)
        .fallback_service(ServeDir::new(static_dir))
}

async fn upgrade_ws(ws: WebSocketUpgrade, State(handle): State<RoomHandle>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, handle))
}

async fn handle_socket(socket: WebSocket, handle: RoomHandle) {
    let mut conn = Conn::default();
    let mut snap_rx = handle.snapshots.subscribe();
    let (mut sink, mut stream) = socket.split();

    match tokio::time::timeout(HELLO_TIMEOUT, next_text(&mut stream)).await {
        Ok(Some(text)) => match serde_json::from_str::<ClientMessage>(&text) {
            Ok(msg @ ClientMessage::Hello { .. }) => {
                if apply_and_reply(&mut sink, &mut conn, msg, &handle)
                    .await
                    .is_err()
                {
                    disconnect(&handle, &conn).await;
                    return;
                }
            }
            _ => {
                let _ = sink.send(policy_close()).await;
                disconnect(&handle, &conn).await;
                return;
            }
        },
        _ => {
            let _ = sink.send(policy_close()).await;
            disconnect(&handle, &conn).await;
            return;
        }
    }

    loop {
        tokio::select! {
            biased;
            incoming = stream.next() => {
                match incoming {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Ok(Message::Text(text))) => {
                        if text.len() > MAX_TEXT_BYTES {
                            continue;
                        }
                        let Ok(msg) = serde_json::from_str::<ClientMessage>(text.as_str()) else {
                            continue;
                        };
                        if conn.role.is_none()
                            && matches!(
                                msg,
                                ClientMessage::Buzz | ClientMessage::Arm | ClientMessage::Reset
                            )
                        {
                            let _ = sink.send(policy_close()).await;
                            break;
                        }
                        if apply_and_reply(&mut sink, &mut conn, msg, &handle)
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            snap = snap_rx.recv() => {
                match snap {
                    Ok(snapshot) => {
                        let msg = snapshot_message(&conn, snapshot, &handle);
                        if send_msg(&mut sink, &msg).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    disconnect(&handle, &conn).await;
}

async fn next_text(stream: &mut SplitStream<WebSocket>) -> Option<String> {
    loop {
        match stream.next().await? {
            Ok(Message::Text(text)) if text.len() <= MAX_TEXT_BYTES => {
                return Some(text.as_str().to_owned());
            }
            Ok(Message::Text(_)) => return None,
            Ok(Message::Ping(_) | Message::Pong(_)) => {}
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(Message::Binary(_)) => return None,
        }
    }
}

async fn apply_and_reply(
    sink: &mut SplitSink<WebSocket, Message>,
    conn: &mut Conn,
    msg: ClientMessage,
    handle: &RoomHandle,
) -> Result<(), ()> {
    if let Some(reply) = apply_client_message(conn, msg, handle).await {
        let hello_ok = matches!(reply, ServerMessage::HelloOk { .. });
        send_msg(sink, &reply).await?;
        if hello_ok {
            if let Some(snapshot) = fetch_snapshot(handle).await {
                send_msg(sink, &snapshot_message(conn, snapshot, handle)).await?;
            }
        }
    }
    Ok(())
}

async fn fetch_snapshot(handle: &RoomHandle) -> Option<Snapshot> {
    let (reply, rx) = oneshot::channel();
    let _ = handle
        .sender()
        .send(Command::GetSnapshot { reply })
        .await;
    rx.await.ok()
}

fn snapshot_message(conn: &Conn, snapshot: Snapshot, handle: &RoomHandle) -> ServerMessage {
    let you = You {
        id: conn.player_id,
        role: conn.role.unwrap_or(ClientRole::Player),
        place: snapshot
            .sequence
            .iter()
            .find(|p| Some(p.player_id) == conn.player_id)
            .map(|p| p.place),
    };
    ServerMessage::from_snapshot(snapshot, you, handle.lan_urls.clone())
}

async fn send_msg(
    sink: &mut SplitSink<WebSocket, Message>,
    msg: &ServerMessage,
) -> Result<(), ()> {
    let text = serde_json::to_string(msg).map_err(|_| ())?;
    sink.send(Message::Text(text.into())).await.map_err(|_| ())
}

fn policy_close() -> Message {
    Message::Close(Some(CloseFrame {
        code: close_code::POLICY,
        reason: "hello required".into(),
    }))
}

async fn disconnect(handle: &RoomHandle, conn: &Conn) {
    let _ = handle
        .sender()
        .send(Command::Disconnect {
            player_id: conn.player_id,
        })
        .await;
}
