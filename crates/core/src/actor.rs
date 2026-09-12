use crate::protocol::ClientRole;
use crate::room::{AuthError, BuzzResult, HelloError, PlayerId, Room, Snapshot};
use tokio::sync::{broadcast, mpsc, oneshot};

#[derive(Debug)]
pub enum Command {
    HelloPlayer {
        resume: Option<PlayerId>,
        name: String,
        reply: oneshot::Sender<Result<PlayerId, HelloError>>,
    },
    HelloHost {
        role: ClientRole,
        host_key: String,
        reply: oneshot::Sender<Result<(), AuthError>>,
    },
    Buzz {
        player_id: PlayerId,
        reply: oneshot::Sender<BuzzResult>,
    },
    Arm {
        host_key: String,
        reply: oneshot::Sender<Result<(), AuthError>>,
    },
    Reset {
        host_key: String,
        reply: oneshot::Sender<Result<(), AuthError>>,
    },
    Disconnect {
        player_id: Option<PlayerId>,
    },
    GetSnapshot {
        reply: oneshot::Sender<Snapshot>,
    },
}

#[derive(Clone)]
pub struct RoomHandle {
    tx: mpsc::Sender<Command>,
    pub snapshots: broadcast::Sender<Snapshot>,
    pub host_key: String,
    pub lan_urls: Vec<String>,
}

impl RoomHandle {
    pub fn spawn(host_key: String, lan_urls: Vec<String>) -> Self {
        let (tx, mut rx) = mpsc::channel::<Command>(1024);
        let (snap_tx, _) = broadcast::channel(64);
        let key_clone = host_key.clone();
        let broadcasts = snap_tx.clone();
        tokio::spawn(async move {
            let mut room = Room::new(key_clone);
            while let Some(cmd) = rx.recv().await {
                let mut changed = true;
                match cmd {
                    Command::HelloPlayer { resume, name, reply } => {
                        let r = room.hello_player(resume, &name).map(|p| p.id);
                        let _ = reply.send(r);
                    }
                    Command::HelloHost { host_key, reply, .. } => {
                        let r = if host_key == room.host_key() {
                            Ok(())
                        } else {
                            Err(AuthError::BadHostKey)
                        };
                        let _ = reply.send(r);
                    }
                    Command::Buzz { player_id, reply } => {
                        let r = room.buzz(player_id);
                        changed = matches!(r, BuzzResult::Accepted { .. });
                        let _ = reply.send(r);
                    }
                    Command::Arm { host_key, reply } => {
                        let r = room.arm(&host_key);
                        let _ = reply.send(r);
                    }
                    Command::Reset { host_key, reply } => {
                        let r = room.reset(&host_key);
                        let _ = reply.send(r);
                    }
                    Command::Disconnect { player_id } => {
                        if let Some(id) = player_id {
                            room.disconnect(id);
                        }
                    }
                    Command::GetSnapshot { reply } => {
                        changed = false;
                        let _ = reply.send(room.snapshot());
                    }
                }
                if changed {
                    let _ = broadcasts.send(room.snapshot());
                }
            }
        });
        Self {
            tx,
            snapshots: snap_tx,
            host_key,
            lan_urls,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<Command> {
        self.tx.clone()
    }
}
