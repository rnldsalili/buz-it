use quiz_buzzer_core::{HelloError, MAX_NAME_CHARS, MAX_PLAYERS, PlayerId, Room};

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
