use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone)]
pub struct Client {
    pub(super) session_expirary: u32,
    pub(super) recv_max: u16,
    pub(super) max_packet_size: u32,
    pub(super) topic_alias_max: u16,
    pub(super) response_info: bool,
    pub(super) problem_info: bool,
    pub(super) clientid: String,
    //global server shutdown
    shutdown: Arc<Notify>,
    // local shutdown signal
    pub(super) killme: Arc<Notify>,
}

impl Client {
    pub(super) fn new(shutdown: Arc<Notify>) -> Self {
        Client {
            session_expirary: 0,
            recv_max: u16::MAX,
            max_packet_size: u32::MAX,
            topic_alias_max: 0,
            response_info: false,
            problem_info: true,
            clientid: String::new(),
            shutdown,
            killme: Arc::new(Notify::new()),
        }
    }

    pub fn shutdown(self) {
        self.shutdown.notify_one();
    }

    pub fn killme(self) {
        self.killme.notify_one();
    }
}
