use crate::ServerError;
use apiformes_packet::prelude::Packet;
use std::sync::Arc;
use tokio::sync::{mpsc::UnboundedSender, Notify};
#[derive(Clone)]
pub struct Client {
    pub(super) session_expirary: u32,
    pub(super) recv_max: u16,
    pub(super) max_packet_size: u32,
    pub(super) topic_alias_max: u16,
    pub(super) response_info: bool,
    pub(super) problem_info: bool,
    pub(super) encrypted: bool,
    pub(super) clientid: Arc<str>,
    //global server shutdown
    pub(super) shutdown: Arc<Notify>,
    // local shutdown signal
    pub(super) killme: Arc<Notify>,
    outgoing: UnboundedSender<Packet>,
}

impl Client {
    pub(super) fn new(
        shutdown: Arc<Notify>,
        outgoing: UnboundedSender<Packet>,
        encrypted: bool,
        max_packet_size: u32,
    ) -> Self {
        Client {
            session_expirary: 0,
            recv_max: u16::MAX,
            max_packet_size,
            topic_alias_max: 0,
            response_info: false,
            problem_info: true,
            clientid: Arc::from(""), //TODO lazy static would be useful here as well
            shutdown,
            killme: Arc::new(Notify::new()),
            outgoing,
            encrypted,
        }
    }

    pub fn encrypted(&self) -> bool {
        self.encrypted
    }
    pub fn shutdown(self) {
        self.shutdown.notify_one();
    }

    pub fn killme(self) {
        self.killme.notify_one();
    }

    pub fn send(&self, packet: Packet) -> Result<(), ServerError> {
        self.outgoing
            .send(packet)
            .map_err(|_| ServerError::Misc("outgoing channel is closed".to_owned()))
    }
}
