//! Core lockout state machine and LAN server for the quiz buzzer.

pub mod actor;
pub mod handler;
pub mod lan;
pub mod protocol;
pub mod room;
pub mod server;

pub use handler::{apply_client_message, Conn};
pub use room::{
    AuthError, BuzzIgnoreReason, BuzzResult, HelloError, Player, PlayerId, Room, Snapshot,
    SnapshotPlace, SnapshotPlayer, MAX_NAME_CHARS, MAX_PLAYERS,
};
pub use server::{bind_server, start_server, ServerConfig};
