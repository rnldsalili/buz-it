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
    };
    apply_client_message(&mut conn, ClientMessage::Arm, &handle).await;
    let snap = get_snapshot(&handle).await;
    assert!(!snap.accepting);
}

#[tokio::test]
async fn clicker_arm_then_player_buzz() {
    let handle = RoomHandle::spawn("secret".into(), vec![]);
    let mut host = Conn {
        role: Some(ClientRole::Clicker),
        player_id: None,
        host_authorized: true,
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
    let mut conn = Conn::default();
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
