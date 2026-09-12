use quiz_buzzer_core::{AuthError, HelloError, MAX_NAME_CHARS, MAX_PLAYERS, PlayerId, Room};

#[test]
fn new_room_is_idle_and_empty() {
    let room = Room::new("hostkeyhostkeyhostkeyhostkey12");
    let snap = room.snapshot();
    assert_eq!(snap.accepting, false);
    assert_eq!(snap.round_id, 0);
    assert!(snap.players.is_empty());
    assert!(snap.sequence.is_empty());
}

#[test]
fn hello_player_assigns_id_and_trims_name() {
    let mut room = Room::new("k");
    let ok = room.hello_player(None, "  Asha  ").unwrap();
    assert_eq!(ok.name, "Asha");
    assert!(room.snapshot().players.iter().any(|p| p.name == "Asha" && p.connected));
}

#[test]
fn hello_player_rejects_empty_and_too_long() {
    let mut room = Room::new("k");
    assert_eq!(room.hello_player(None, "   ").unwrap_err(), HelloError::BadName);
    let long = "a".repeat(MAX_NAME_CHARS + 1);
    assert_eq!(room.hello_player(None, &long).unwrap_err(), HelloError::BadName);
}

#[test]
fn hello_player_reconnects_same_id() {
    let mut room = Room::new("k");
    let first = room.hello_player(None, "Asha").unwrap();
    room.disconnect(first.id);
    assert!(!room.snapshot().players.iter().find(|p| p.id == first.id).unwrap().connected);
    let again = room.hello_player(Some(first.id), "Asha 2").unwrap();
    assert_eq!(again.id, first.id);
    let p = room.snapshot().players.into_iter().find(|p| p.id == first.id).unwrap();
    assert!(p.connected);
    assert_eq!(p.name, "Asha 2");
}

#[test]
fn hello_player_unknown_id_mints_new() {
    let mut room = Room::new("k");
    let ghost = PlayerId::nil();
    let ok = room.hello_player(Some(ghost), "Bea").unwrap();
    assert_ne!(ok.id, ghost);
}

#[test]
fn hello_player_room_full() {
    let mut room = Room::new("k");
    for i in 0..MAX_PLAYERS {
        room.hello_player(None, &format!("p{i}")).unwrap();
    }
    assert_eq!(room.hello_player(None, "overflow").unwrap_err(), HelloError::RoomFull);
}

#[test]
fn duplicate_names_allowed() {
    let mut room = Room::new("k");
    room.hello_player(None, "Asha").unwrap();
    room.hello_player(None, "Asha").unwrap();
    assert_eq!(room.snapshot().players.len(), 2);
}

#[test]
fn hello_player_frees_slot_when_disconnected() {
    let mut room = Room::new("k");
    let mut ids = Vec::new();
    for i in 0..MAX_PLAYERS {
        ids.push(room.hello_player(None, &format!("p{i}")).unwrap().id);
    }
    assert_eq!(room.hello_player(None, "overflow").unwrap_err(), HelloError::RoomFull);

    room.disconnect(ids[0]);
    let joined = room.hello_player(None, "newbie").unwrap();
    assert_eq!(joined.name, "newbie");
    assert!(joined.connected);

    // Room is full of connected players again; resume of an existing id still works.
    let resumed = room.hello_player(Some(ids[1]), "renamed").unwrap();
    assert_eq!(resumed.id, ids[1]);
    assert_eq!(resumed.name, "renamed");
    assert!(resumed.connected);
}

#[test]
fn arm_requires_host_key() {
    let mut room = Room::new("secret");
    assert_eq!(room.arm("nope"), Err(AuthError::BadHostKey));
    room.arm("secret").unwrap();
    assert!(room.snapshot().accepting);
    assert_eq!(room.snapshot().round_id, 1);
}

#[test]
fn arm_clears_sequence_and_increments_round() {
    let mut room = Room::new("secret");
    let a = room.hello_player(None, "A").unwrap();
    room.arm("secret").unwrap();
    room.buzz(a.id);
    room.arm("secret").unwrap();
    let snap = room.snapshot();
    assert!(snap.sequence.is_empty());
    assert_eq!(snap.round_id, 2);
    assert!(snap.accepting);
}

#[test]
fn reset_freezes_but_keeps_sequence() {
    let mut room = Room::new("secret");
    let a = room.hello_player(None, "A").unwrap();
    room.arm("secret").unwrap();
    room.buzz(a.id);
    room.reset("secret").unwrap();
    let snap = room.snapshot();
    assert_eq!(snap.accepting, false);
    assert_eq!(snap.sequence.len(), 1);
    assert_eq!(room.reset("nope"), Err(AuthError::BadHostKey));
}

use quiz_buzzer_core::{BuzzIgnoreReason, BuzzResult};

#[test]
fn buzz_ignored_when_idle() {
    let mut room = Room::new("secret");
    let a = room.hello_player(None, "A").unwrap();
    assert_eq!(
        room.buzz(a.id),
        BuzzResult::Ignored {
            reason: BuzzIgnoreReason::NotAccepting
        }
    );
    assert!(room.snapshot().sequence.is_empty());
}

#[test]
fn first_buzz_wins_and_later_players_append() {
    let mut room = Room::new("secret");
    let a = room.hello_player(None, "A").unwrap();
    let b = room.hello_player(None, "B").unwrap();
    let c = room.hello_player(None, "C").unwrap();
    room.arm("secret").unwrap();
    assert_eq!(room.buzz(b.id), BuzzResult::Accepted { place: 1, first: true });
    assert_eq!(room.buzz(a.id), BuzzResult::Accepted { place: 2, first: false });
    assert_eq!(room.buzz(c.id), BuzzResult::Accepted { place: 3, first: false });
    let seq: Vec<_> = room.snapshot().sequence.iter().map(|s| s.player_id).collect();
    assert_eq!(seq, vec![b.id, a.id, c.id]);
    assert_eq!(room.snapshot().sequence[0].place, 1);
    assert_eq!(room.snapshot().sequence[0].name, "B");
}

#[test]
fn second_buzz_from_same_player_ignored() {
    let mut room = Room::new("secret");
    let a = room.hello_player(None, "A").unwrap();
    room.arm("secret").unwrap();
    room.buzz(a.id);
    assert_eq!(
        room.buzz(a.id),
        BuzzResult::Ignored {
            reason: BuzzIgnoreReason::AlreadyBuzzed
        }
    );
    assert_eq!(room.snapshot().sequence.len(), 1);
}

#[test]
fn unknown_player_cannot_buzz() {
    let mut room = Room::new("secret");
    room.arm("secret").unwrap();
    assert_eq!(
        room.buzz(PlayerId::new_v4()),
        BuzzResult::Ignored {
            reason: BuzzIgnoreReason::UnknownPlayer
        }
    );
}

#[test]
fn disconnected_player_still_keeps_place() {
    let mut room = Room::new("secret");
    let a = room.hello_player(None, "A").unwrap();
    room.arm("secret").unwrap();
    room.buzz(a.id);
    room.disconnect(a.id);
    assert_eq!(room.snapshot().sequence[0].player_id, a.id);
    assert!(!room.snapshot().players.iter().find(|p| p.id == a.id).unwrap().connected);
}
