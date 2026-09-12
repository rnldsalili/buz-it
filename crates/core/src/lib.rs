//! Core lockout state machine and LAN server for the quiz buzzer.

pub mod actor;
pub mod lan;
pub mod protocol;
pub mod room;

pub use room::{
    AuthError, BuzzIgnoreReason, BuzzResult, HelloError, Player, PlayerId, Room, Snapshot,
    SnapshotPlace, SnapshotPlayer, MAX_NAME_CHARS, MAX_PLAYERS,
};
