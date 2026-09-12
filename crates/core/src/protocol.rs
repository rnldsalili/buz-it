use crate::room::{PlayerId, Snapshot};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "hello")]
    Hello {
        role: ClientRole,
        #[serde(default)]
        name: Option<String>,
        #[serde(default, rename = "playerId")]
        player_id: Option<PlayerId>,
        #[serde(default, rename = "hostKey")]
        host_key: Option<String>,
    },
    #[serde(rename = "buzz")]
    Buzz,
    #[serde(rename = "arm")]
    Arm,
    #[serde(rename = "reset")]
    Reset,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClientRole {
    Player,
    Board,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "helloOk")]
    HelloOk {
        #[serde(rename = "playerId")]
        player_id: Option<PlayerId>,
        role: ClientRole,
    },
    #[serde(rename = "error")]
    Error { code: String, message: String },
    #[serde(rename = "snapshot")]
    Snapshot {
        accepting: bool,
        #[serde(rename = "roundId")]
        round_id: u64,
        players: Vec<crate::room::SnapshotPlayer>,
        sequence: Vec<crate::room::SnapshotPlace>,
        #[serde(rename = "lanUrls")]
        lan_urls: Vec<String>,
        you: You,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct You {
    pub id: Option<PlayerId>,
    pub role: ClientRole,
    pub place: Option<u32>,
}

impl ServerMessage {
    pub fn from_snapshot(snapshot: Snapshot, you: You, lan_urls: Vec<String>) -> Self {
        ServerMessage::Snapshot {
            accepting: snapshot.accepting,
            round_id: snapshot.round_id,
            players: snapshot.players,
            sequence: snapshot.sequence,
            lan_urls,
            you,
        }
    }
}
