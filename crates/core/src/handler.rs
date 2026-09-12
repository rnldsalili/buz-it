use std::net::SocketAddr;

use crate::actor::{Command, RoomHandle};
use crate::protocol::{ClientMessage, ClientRole, ServerMessage};
use crate::room::{HelloError, PlayerId};
use tokio::sync::oneshot;

#[derive(Clone, Debug, Default)]
pub struct Conn {
    pub role: Option<ClientRole>,
    pub player_id: Option<PlayerId>,
    pub host_authorized: bool,
    pub peer_addr: Option<SocketAddr>,
}

pub async fn apply_client_message(
    conn: &mut Conn,
    msg: ClientMessage,
    handle: &RoomHandle,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Hello {
            role,
            name,
            player_id,
            host_key,
        } => {
            // A new hello always revokes the old role, including failed handshakes.
            conn.host_authorized = false;
            conn.role = None;
            if let Some(player_id) = conn.player_id.take() {
                let _ = handle
                    .sender()
                    .send(Command::Disconnect {
                        player_id: Some(player_id),
                    })
                    .await;
            }
            match role {
                ClientRole::Player => hello_player(conn, name, player_id, handle).await,
                ClientRole::Board => hello_host(conn, role, host_key, handle).await,
            }
        }
        ClientMessage::Buzz => {
            if conn.role != Some(ClientRole::Player) {
                return None;
            }
            let Some(player_id) = conn.player_id else {
                return None;
            };
            let (reply, rx) = oneshot::channel();
            let _ = handle
                .sender()
                .send(Command::Buzz { player_id, reply })
                .await;
            let _ = rx.await;
            None
        }
        ClientMessage::Arm => {
            if !conn.host_authorized
                || conn.role != Some(ClientRole::Board)
                || !conn.peer_addr.is_some_and(|peer| peer.ip().is_loopback())
            {
                return None;
            }
            let (reply, rx) = oneshot::channel();
            let _ = handle
                .sender()
                .send(Command::Arm {
                    host_key: handle.host_key.clone(),
                    reply,
                })
                .await;
            let _ = rx.await;
            None
        }
        ClientMessage::Reset => {
            if !conn.host_authorized
                || conn.role != Some(ClientRole::Board)
                || !conn.peer_addr.is_some_and(|peer| peer.ip().is_loopback())
            {
                return None;
            }
            let (reply, rx) = oneshot::channel();
            let _ = handle
                .sender()
                .send(Command::Reset {
                    host_key: handle.host_key.clone(),
                    reply,
                })
                .await;
            let _ = rx.await;
            None
        }
    }
}

async fn hello_player(
    conn: &mut Conn,
    name: Option<String>,
    resume: Option<PlayerId>,
    handle: &RoomHandle,
) -> Option<ServerMessage> {
    let (reply, rx) = oneshot::channel();
    let _ = handle
        .sender()
        .send(Command::HelloPlayer {
            resume,
            name: name.unwrap_or_default(),
            reply,
        })
        .await;
    match rx.await {
        Ok(Ok(id)) => {
            conn.role = Some(ClientRole::Player);
            conn.player_id = Some(id);
            Some(ServerMessage::HelloOk {
                player_id: Some(id),
                role: ClientRole::Player,
            })
        }
        Ok(Err(HelloError::BadName)) => Some(ServerMessage::Error {
            code: "bad_name".into(),
            message: "Name required".into(),
        }),
        Ok(Err(HelloError::RoomFull)) => Some(ServerMessage::Error {
            code: "room_full".into(),
            message: "Room is full".into(),
        }),
        Err(_) => None,
    }
}

async fn hello_host(
    conn: &mut Conn,
    role: ClientRole,
    host_key: Option<String>,
    handle: &RoomHandle,
) -> Option<ServerMessage> {
    if !conn.peer_addr.is_some_and(|peer| peer.ip().is_loopback()) {
        return Some(ServerMessage::Error {
            code: "local_board_only".into(),
            message: "Open Buz It on the host laptop to control rounds.".into(),
        });
    }
    let (reply, rx) = oneshot::channel();
    let _ = handle
        .sender()
        .send(Command::HelloHost {
            role,
            host_key: host_key.unwrap_or_default(),
            reply,
        })
        .await;
    match rx.await {
        Ok(Ok(())) => {
            conn.role = Some(role);
            conn.host_authorized = true;
            Some(ServerMessage::HelloOk {
                player_id: None,
                role,
            })
        }
        Ok(Err(_)) => Some(ServerMessage::Error {
            code: "bad_host_key".into(),
            message: "Invalid host key".into(),
        }),
        Err(_) => None,
    }
}
