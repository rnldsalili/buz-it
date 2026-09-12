use quiz_buzzer_core::protocol::{ClientMessage, ClientRole};

#[test]
fn parses_player_hello() {
    let msg: ClientMessage = serde_json::from_str(
        r#"{"type":"hello","role":"player","name":"Asha","playerId":null}"#,
    )
    .unwrap();
    match msg {
        ClientMessage::Hello { role, name, .. } => {
            assert_eq!(role, ClientRole::Player);
            assert_eq!(name.as_deref(), Some("Asha"));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn parses_buzz() {
    let msg: ClientMessage = serde_json::from_str(r#"{"type":"buzz"}"#).unwrap();
    assert!(matches!(msg, ClientMessage::Buzz));
}

#[test]
fn snapshot_place_uses_player_id_camel_case() {
    let place = quiz_buzzer_core::SnapshotPlace {
        player_id: quiz_buzzer_core::PlayerId::nil(),
        name: "A".into(),
        place: 1,
    };
    let v = serde_json::to_value(&place).unwrap();
    assert!(v.get("playerId").is_some());
    assert!(v.get("player_id").is_none());
}
