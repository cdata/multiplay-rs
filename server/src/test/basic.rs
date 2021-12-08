use std::sync::Arc;

use async_std::sync::Mutex;

use serial_test::serial;

use crate::{
    rtc::{
        protocol::server::OutgoingMessage,
        server::RtcServer,
        session::{Session, Transport},
    },
    test::helpers::TestProtocol,
};

use super::helpers;

#[tokio::test]
#[serial]
async fn messages_forward_to_transport() {
    let rtc_server = RtcServer::new();
    let message_count = Arc::new(Mutex::new(0));
    let sessions = rtc_server.get_sessions();
    let mut session = Session::new();

    session.authenticated = true;
    session.transports.insert(Transport::Bulk);

    sessions.lock().await.insert(session.id, session);

    let (tx, _rx) = rtc_server.game_channel();

    let message_sends = tx.send_async(OutgoingMessage::Broadcast(
        serde_cbor::ser::to_vec(&TestProtocol::Stop).unwrap(),
    ));

    let rtc_server_runs = rtc_server.run(vec![helpers::TestTransport::with_state(
        message_count.clone(),
    )]);

    let result = futures::future::join(message_sends, rtc_server_runs).await;

    match result {
        (Err(error), _) => info!("Message send: {:?}", error),
        (_, Err(error)) => info!("RTCServer: {:?}", error),
        _ => (),
    };

    assert_eq!(*message_count.lock().await, 1);
}
