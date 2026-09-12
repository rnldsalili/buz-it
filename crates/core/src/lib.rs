//! Core lockout state machine and LAN server for the quiz buzzer.

pub mod room;

pub use room::{
    BuzzIgnoreReason, BuzzResult, Player, PlayerId, Room, Snapshot, SnapshotPlace, SnapshotPlayer,
};
