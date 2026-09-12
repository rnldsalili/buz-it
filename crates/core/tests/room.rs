use quiz_buzzer_core::Room;

#[test]
fn new_room_is_idle_and_empty() {
    let room = Room::new("hostkeyhostkeyhostkeyhostkey12");
    let snap = room.snapshot();
    assert_eq!(snap.accepting, false);
    assert_eq!(snap.round_id, 0);
    assert!(snap.players.is_empty());
    assert!(snap.sequence.is_empty());
}
