use quiz_buzzer_core::actor::{Command, RoomHandle};
use quiz_buzzer_core::{BuzzResult, PlayerId};
use tokio::sync::oneshot;

async fn hello(handle: &RoomHandle, name: &str) -> PlayerId {
    let (reply, rx) = oneshot::channel();
    handle
        .sender()
        .send(Command::HelloPlayer {
            resume: None,
            name: name.into(),
            reply,
        })
        .await
        .unwrap();
    rx.await.unwrap().unwrap()
}

#[tokio::test]
async fn buzz_order_matches_enqueue_order() {
    let handle = RoomHandle::spawn("secret".into(), vec![]);
    let a = hello(&handle, "A").await;
    let b = hello(&handle, "B").await;
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

    let (ra, rxa) = oneshot::channel();
    let (rb, rxb) = oneshot::channel();
    let tx = handle.sender();
    tx.send(Command::Buzz {
        player_id: b,
        reply: rb,
    })
    .await
    .unwrap();
    tx.send(Command::Buzz {
        player_id: a,
        reply: ra,
    })
    .await
    .unwrap();
    assert_eq!(rxb.await.unwrap(), BuzzResult::Accepted { place: 1, first: true });
    assert_eq!(rxa.await.unwrap(), BuzzResult::Accepted { place: 2, first: false });
}
