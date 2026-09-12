use quiz_buzzer_core::actor::{Command, RoomHandle};
use quiz_buzzer_core::handler::{apply_client_message, Conn};
use quiz_buzzer_core::protocol::{ClientMessage, ClientRole, ServerMessage};
use quiz_buzzer_core::room::Snapshot;
use tokio::sync::oneshot;

async fn get_snapshot(handle: &RoomHandle) -> Snapshot {
    let (reply, rx) = oneshot::channel();
    handle
        .sender()
        .send(Command::GetSnapshot { reply })
        .await
        .unwrap();
    rx.await.unwrap()
}

#[tokio::test]
async fn player_cannot_arm() {
    let handle = RoomHandle::spawn("secret".into(), vec![]);
    let mut conn = Conn {
        role: Some(ClientRole::Player),
        player_id: Some(quiz_buzzer_core::PlayerId::nil()),
        host_authorized: false,
        ..Conn::default()
    };
    apply_client_message(&mut conn, ClientMessage::Arm, &handle).await;
    let snap = get_snapshot(&handle).await;
    assert!(!snap.accepting);
}

#[tokio::test]
async fn local_board_arm_then_player_buzz() {
    let handle = RoomHandle::spawn("secret".into(), vec![]);
    let mut host = Conn {
        role: Some(ClientRole::Board),
        player_id: None,
        host_authorized: true,
        peer_addr: Some("127.0.0.1:12345".parse().unwrap()),
    };
    apply_client_message(&mut host, ClientMessage::Arm, &handle).await;

    let mut player_conn = Conn::default();
    let id = match apply_client_message(
        &mut player_conn,
        ClientMessage::Hello {
            role: ClientRole::Player,
            name: Some("A".into()),
            player_id: None,
            host_key: None,
        },
        &handle,
    )
    .await
    {
        Some(ServerMessage::HelloOk { player_id, .. }) => player_id.unwrap(),
        other => panic!("{other:?}"),
    };
    apply_client_message(&mut player_conn, ClientMessage::Buzz, &handle).await;
    let snap = get_snapshot(&handle).await;
    assert_eq!(snap.sequence[0].place, 1);
    assert_eq!(snap.sequence[0].player_id, id);
}

#[tokio::test]
async fn bad_host_key_rejected() {
    let handle = RoomHandle::spawn("secret".into(), vec![]);
    let mut conn = Conn {
        peer_addr: Some("127.0.0.1:12345".parse().unwrap()),
        ..Conn::default()
    };
    let msg = apply_client_message(
        &mut conn,
        ClientMessage::Hello {
            role: ClientRole::Board,
            name: None,
            player_id: None,
            host_key: Some("nope".into()),
        },
        &handle,
    )
    .await;
    match msg {
        Some(ServerMessage::Error { code, .. }) => assert_eq!(code, "bad_host_key"),
        other => panic!("{other:?}"),
    }
    assert!(!conn.host_authorized);
}

#[tokio::test]
async fn board_without_verified_loopback_peer_is_rejected_even_with_valid_key() {
    let handle = RoomHandle::spawn("secret".into(), vec![]);
    let mut conn = Conn::default();
    let reply = apply_client_message(
        &mut conn,
        ClientMessage::Hello {
            role: ClientRole::Board,
            name: None,
            player_id: None,
            host_key: Some("secret".into()),
        },
        &handle,
    )
    .await;
    assert!(matches!(reply, Some(ServerMessage::Error { .. })));
    apply_client_message(&mut conn, ClientMessage::Arm, &handle).await;
    assert!(!get_snapshot(&handle).await.accepting);
}

#[tokio::test]
async fn changing_to_player_revokes_host_authorization() {
    let handle = RoomHandle::spawn("secret".into(), vec![]);
    let mut conn = Conn {
        role: Some(ClientRole::Board),
        host_authorized: true,
        peer_addr: Some("127.0.0.1:12345".parse().unwrap()),
        ..Conn::default()
    };
    apply_client_message(
        &mut conn,
        ClientMessage::Hello {
            role: ClientRole::Player,
            name: Some("A".into()),
            player_id: None,
            host_key: None,
        },
        &handle,
    )
    .await;
    assert!(!conn.host_authorized);
    apply_client_message(&mut conn, ClientMessage::Arm, &handle).await;
    assert!(!get_snapshot(&handle).await.accepting);
}

#[test]
fn old_clicker_protocol_is_rejected() {
    assert!(serde_json::from_str::<ClientMessage>(
        r#"{"type":"hello","role":"clicker","hostKey":"secret"}"#
    )
    .is_err());
}

#[tokio::test]
async fn lan_board_key_cannot_authorize_or_control_rounds() {
    let handle = RoomHandle::spawn("secret".into(), vec![]);
    let mut conn = Conn {
        peer_addr: Some("192.168.1.20:12345".parse().unwrap()),
        ..Conn::default()
    };
    let reply = apply_client_message(
        &mut conn,
        ClientMessage::Hello {
            role: ClientRole::Board,
            name: None,
            player_id: None,
            host_key: Some("secret".into()),
        },
        &handle,
    )
    .await;
    assert!(matches!(reply, Some(ServerMessage::Error { code, .. }) if code == "local_board_only"));
    apply_client_message(&mut conn, ClientMessage::Arm, &handle).await;
    assert!(!get_snapshot(&handle).await.accepting);
}

#[tokio::test]
async fn local_board_handshake_and_failed_reauthentication() {
    for peer in ["127.0.0.1:12345", "[::1]:12345"] {
        let handle = RoomHandle::spawn("secret".into(), vec![]);
        let mut conn = Conn {
            peer_addr: Some(peer.parse().unwrap()),
            ..Conn::default()
        };
        let hello = |key: &str| ClientMessage::Hello {
            role: ClientRole::Board,
            name: None,
            player_id: None,
            host_key: Some(key.into()),
        };
        assert!(matches!(
            apply_client_message(&mut conn, hello("secret"), &handle).await,
            Some(ServerMessage::HelloOk { .. })
        ));
        apply_client_message(&mut conn, ClientMessage::Arm, &handle).await;
        assert!(get_snapshot(&handle).await.accepting);
        apply_client_message(&mut conn, hello("wrong"), &handle).await;
        assert!(!conn.host_authorized);
        apply_client_message(&mut conn, ClientMessage::Reset, &handle).await;
        assert!(get_snapshot(&handle).await.accepting);
    }
}

#[tokio::test]
async fn every_round_command_checks_role_and_peer_despite_stale_authorized_flag() {
    for (role, peer) in [
        (ClientRole::Player, "127.0.0.1:12345"),
        (ClientRole::Board, "192.168.1.20:12345"),
    ] {
        let handle = RoomHandle::spawn("secret".into(), vec![]);
        let mut conn = Conn {
            role: Some(role),
            host_authorized: true,
            peer_addr: Some(peer.parse().unwrap()),
            ..Conn::default()
        };
        apply_client_message(&mut conn, ClientMessage::Arm, &handle).await;
        assert!(!get_snapshot(&handle).await.accepting);
        let (reply, rx) = oneshot::channel();
        handle
            .sender()
            .send(Command::Arm {
                host_key: "secret".into(),
                reply,
            })
            .await
            .unwrap();
        rx.await.unwrap().unwrap();
        apply_client_message(&mut conn, ClientMessage::Reset, &handle).await;
        assert!(get_snapshot(&handle).await.accepting);
    }
}
