use std::{net::SocketAddr, sync::Arc};

use async_std::sync::Mutex;
use flume::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use tokio::task;
use tokio::task::JoinHandle;
use warp;
use warp::http::Method;
use warp::Filter;

use crate::rtc::protocol::{IncomingMessage, OutgoingMessage};
use crate::rtc::session::Sessions;

use std::fmt::Debug;

mod handlers {
    use std::{net::SocketAddr, sync::Arc};

    use async_std::sync::Mutex;
    use serde::{Deserialize, Serialize};
    use std::fmt::Debug;
    use warp::{Rejection, Reply};

    use flume::Sender;

    use crate::rtc::protocol::{IncomingMessage, SessionControlMessage, SessionPayload};
    use crate::rtc::session::{Session, Sessions};

    pub async fn create_session<T>(
        shared_sessions: Arc<Mutex<Sessions>>,
        send_to_game: Sender<IncomingMessage<T>>,
        socket_addr: Option<SocketAddr>,
    ) -> Result<impl Reply, Rejection>
    where
        T: Deserialize<'static> + Serialize + Send + Sync + Debug,
    {
        debug!("New session requested by {:?}...", socket_addr);

        let session = Session::new();
        let id = session.id;

        let (auth_sender, auth_receiver) = flume::unbounded::<SessionControlMessage>();

        match send_to_game
            .send_async(IncomingMessage::Solicited(id, socket_addr, auth_sender))
            .await
        {
            Ok(_) => match auth_receiver.recv_async().await {
                Ok(control_message) => match control_message {
                    SessionControlMessage::Accept(_) => {
                        let mut sessions = shared_sessions.lock().await;
                        sessions.insert(id, session);
                        Ok(warp::reply::json(&SessionPayload { id }))
                    }
                    SessionControlMessage::Kick(_) => Err(warp::reject()),
                },
                Err(error) => {
                    error!("Error waiting for session authentication: {:?}", error);
                    Err(warp::reject())
                }
            },
            Err(error) => {
                error!("Error requesting session authentication: {:?}", error);
                Err(warp::reject())
            }
        }
    }
}

async fn serve_rtc_sessions_task<T>(
    shared_sessions: Arc<Mutex<Sessions>>,
    address: SocketAddr,
    send_to_game: Sender<IncomingMessage<T>>,
) where
    T: 'static + Deserialize<'static> + Serialize + Send + Sync + Debug,
{
    let with_sessions = warp::any().map(move || shared_sessions.clone());
    let with_send_to_game = warp::any().map(move || send_to_game.clone());

    let create_session = warp::path!("session")
        .and(warp::post())
        .and(with_sessions)
        .and(with_send_to_game)
        .and(warp::filters::addr::remote())
        .and_then(handlers::create_session);

    let routes = warp::any().and(create_session).with(
        warp::cors()
            .allow_method(Method::POST)
            .allow_origin("http://localhost:8000"),
    );

    warp::serve(routes).run(address).await;
}

pub fn serve_rtc_sessions<T>(
    shared_sessions: Arc<Mutex<Sessions>>,
    address: SocketAddr,
    send_to_game: Sender<IncomingMessage<T>>,
    _receive_from_game: Receiver<OutgoingMessage<T>>,
) -> JoinHandle<()>
where
    T: 'static + Deserialize<'static> + Serialize + Send + Sync + Debug,
{
    task::spawn(serve_rtc_sessions_task(
        shared_sessions.clone(),
        address,
        send_to_game.clone(),
    ))
}
