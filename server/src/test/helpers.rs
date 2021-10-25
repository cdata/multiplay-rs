use std::sync::Arc;

use async_std::sync::Mutex;

use crate::rtc::session::Sessions;

pub fn init_logging() {
    // NOTE(cdata): pretty_env_logger's builder seems busted
    // See: https://github.com/seanmonstar/pretty-env-logger/issues/43
    pretty_env_logger::env_logger::Builder::from_default_env()
        .is_test(true)
        .init();
}

pub fn make_empty_session_state() -> Arc<Mutex<Sessions>> {
    Arc::new(Mutex::new(Sessions::new()))
}
