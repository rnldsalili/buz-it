use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub type PlayerId = Uuid;

pub const MAX_PLAYERS: usize = 250;
pub const MAX_NAME_CHARS: usize = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HelloError {
    BadName,
    RoomFull,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthError {
    BadHostKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub connected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotPlayer {
    pub id: PlayerId,
    pub name: String,
    pub connected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotPlace {
    pub player_id: PlayerId,
    pub name: String,
    pub place: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub accepting: bool,
    pub round_id: u64,
    pub players: Vec<SnapshotPlayer>,
    pub sequence: Vec<SnapshotPlace>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuzzIgnoreReason {
    NotAccepting,
    AlreadyBuzzed,
    UnknownPlayer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuzzResult {
    Ignored { reason: BuzzIgnoreReason },
    Accepted { place: u32, first: bool },
}

#[derive(Debug)]
pub struct Room {
    pub(crate) host_key: String,
    accepting: bool,
    round_id: u64,
    players: HashMap<PlayerId, Player>,
    sequence: Vec<PlayerId>,
}

impl Room {
    pub fn new(host_key: impl Into<String>) -> Self {
        Self {
            host_key: host_key.into(),
            accepting: false,
            round_id: 0,
            players: HashMap::new(),
            sequence: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let players = self
            .players
            .values()
            .map(|p| SnapshotPlayer {
                id: p.id,
                name: p.name.clone(),
                connected: p.connected,
            })
            .collect();
        let sequence = self
            .sequence
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                self.players.get(id).map(|p| SnapshotPlace {
                    player_id: *id,
                    name: p.name.clone(),
                    place: (i as u32) + 1,
                })
            })
            .collect();
        Snapshot {
            accepting: self.accepting,
            round_id: self.round_id,
            players,
            sequence,
        }
    }

    pub fn host_key(&self) -> &str {
        &self.host_key
    }

    pub fn hello_player(
        &mut self,
        resume: Option<PlayerId>,
        name: &str,
    ) -> Result<Player, HelloError> {
        let name = normalize_name(name)?;
        if let Some(id) = resume {
            if let Some(existing) = self.players.get_mut(&id) {
                existing.name = name;
                existing.connected = true;
                return Ok(existing.clone());
            }
        }
        if self.players.values().filter(|p| p.connected).count() >= MAX_PLAYERS {
            return Err(HelloError::RoomFull);
        }
        let player = Player {
            id: Uuid::new_v4(),
            name,
            connected: true,
        };
        self.players.insert(player.id, player.clone());
        Ok(player)
    }

    pub fn disconnect(&mut self, id: PlayerId) {
        if let Some(p) = self.players.get_mut(&id) {
            p.connected = false;
        }
    }

    fn check_host_key(&self, key: &str) -> Result<(), AuthError> {
        if key == self.host_key {
            Ok(())
        } else {
            Err(AuthError::BadHostKey)
        }
    }

    pub fn arm(&mut self, host_key: &str) -> Result<(), AuthError> {
        self.check_host_key(host_key)?;
        self.sequence.clear();
        self.accepting = true;
        self.round_id += 1;
        Ok(())
    }

    pub fn reset(&mut self, host_key: &str) -> Result<(), AuthError> {
        self.check_host_key(host_key)?;
        self.accepting = false;
        Ok(())
    }

    pub fn buzz(&mut self, player_id: PlayerId) -> BuzzResult {
        if !self.accepting {
            return BuzzResult::Ignored {
                reason: BuzzIgnoreReason::NotAccepting,
            };
        }
        if !self.players.contains_key(&player_id) {
            return BuzzResult::Ignored {
                reason: BuzzIgnoreReason::UnknownPlayer,
            };
        }
        if self.sequence.contains(&player_id) {
            return BuzzResult::Ignored {
                reason: BuzzIgnoreReason::AlreadyBuzzed,
            };
        }
        self.sequence.push(player_id);
        let place = self.sequence.len() as u32;
        BuzzResult::Accepted {
            place,
            first: place == 1,
        }
    }
}

fn normalize_name(raw: &str) -> Result<String, HelloError> {
    let name: String = raw.trim().chars().collect();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(HelloError::BadName);
    }
    Ok(name)
}
