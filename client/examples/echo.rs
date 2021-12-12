use std::time::Duration;

use multiplay_rs::rtc::{server::RtcServer, transport::websockets::WebSocketTransport};

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod echo {
    use anyhow::Result;
    use flume::{Receiver, Sender};
    use multiplay_rs::rtc::protocol::server::{
        IncomingMessage, OutgoingMessage, SessionControlMessage,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Message {
        Echo(String),
    }

    pub struct Echo {
        tx: Sender<OutgoingMessage>,
        rx: Receiver<IncomingMessage>,
    }

    impl Echo {
        pub fn new(tx: Sender<OutgoingMessage>, rx: Receiver<IncomingMessage>) -> Self {
            Echo { tx, rx }
        }

        pub async fn run(self) -> Result<()> {
            let tx = self.tx;
            let rx = self.rx;

            while let Ok(message) = rx.recv_async().await {
                match message {
                    IncomingMessage::Solicited(_, auth_tx) => {
                        auth_tx
                            .send_async(SessionControlMessage::Accept(None))
                            .await?
                    }
                    IncomingMessage::Received(session_id, data) => {
                        match serde_cbor::de::from_slice(&data)? {
                            Message::Echo(echo_message) => {
                                println!("Got client message: {:?}", echo_message);
                                println!("Sending response to client...");
                                tx.send_async(OutgoingMessage::Send(session_id, data))
                                    .await?;
                            }
                        }
                    }
                    _ => (),
                }
            }

            Ok(())
        }
    }
}

#[tokio::main]
pub async fn main() {
    pretty_env_logger::init();

    info!("Starting echo example...");

    let server = RtcServer::new();
    let channel = server.game_channel();
    let server_runs = server.run(vec![WebSocketTransport::new(([127, 0, 0, 1], 8001).into())]);

    let echo = echo::Echo::new(channel.0, channel.1);
    let echo_runs = echo.run();

    let client_runs = async {
        let mut client = multiplay_rs_client::Client::new();
        let (tx, rx) = client.connect::<echo::Message>("ws://127.0.0.1:8001");

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Sending message to server...");

            tx.send_async(echo::Message::Echo(String::from("Hello there!")))
                .await
                .unwrap();

            println!("Awaiting response from server...");
            let response = rx.recv_async().await.unwrap();

            println!("Got message from server: {:?}", response);
        }
    };

    tokio::select! {
      _ = server_runs => info!("Server has stopped"),
      _ = echo_runs => info!("Echo has stopped"),
      _ = client_runs => info!("Client has stopped")
    }

    info!("Echo example over!");
}
