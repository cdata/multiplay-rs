use std::sync::Arc;

use async_std::sync::Mutex;
use flume::Sender;
use warp;
use warp::http::Method;
use warp::Filter;

use crate::rtc::protocol::server::IncomingMessage;
use crate::rtc::session::Sessions;

mod handlers {
    use std::{net::SocketAddr, sync::Arc};

    use async_std::sync::Mutex;
    use warp::{Rejection, Reply};

    use flume::Sender;

    use crate::rtc::protocol::server::{IncomingMessage, SessionControlMessage, SessionPayload};
    use crate::rtc::session::{Session, Sessions};

    pub(crate) async fn create_session(
        shared_sessions: Arc<Mutex<Sessions>>,
        send_to_game: Sender<IncomingMessage>,
        socket_addr: Option<SocketAddr>,
    ) -> Result<impl Reply, Rejection> {
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

pub fn sessions_api(
    shared_sessions: Arc<Mutex<Sessions>>,
    send_to_game: Sender<IncomingMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let with_sessions = warp::any().map(move || shared_sessions.clone());
    let with_send_to_game = warp::any().map(move || send_to_game.clone());

    let create_session = warp::path!("session")
        .and(warp::post())
        .and(with_sessions)
        .and(with_send_to_game)
        .and(warp::filters::addr::remote())
        .and_then(handlers::create_session);

    warp::any().and(create_session).with(
        warp::cors()
            .allow_method(Method::POST)
            .allow_origin("http://localhost:8000"),
    )
}
