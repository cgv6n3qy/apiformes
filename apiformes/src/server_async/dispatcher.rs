use super::{topics::Topics, Client, MqttServerConfig};
use tokio::sync::{mpsc::UnboundedReceiver, Notify, RwLock};
use tokio::task::JoinHandle;

use crate::packets::prelude::Packet;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, instrument, warn};

pub struct Dispatcher {
    topics: Arc<RwLock<Topics>>,
    cfg: Arc<MqttServerConfig>,
    shutdown: Arc<Notify>,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    incoming: UnboundedReceiver<(String, Packet)>,
}

impl Dispatcher {
    pub fn new(
        topics: Arc<RwLock<Topics>>,
        cfg: Arc<MqttServerConfig>,
        shutdown: Arc<Notify>,
        clients: Arc<RwLock<HashMap<String, Client>>>,
        incoming: UnboundedReceiver<(String, Packet)>,
    ) -> Self {
        Dispatcher {
            topics,
            cfg,
            shutdown,
            clients,
            incoming,
        }
    }

    async fn process_packet(&mut self, client: String, _packet: Packet) {
        error!(
            cliendid = &*client,
            "received packet which is not supported"
        );
    }
    async fn process_forever(mut self) {
        loop {
            let (client, packet) = match self.incoming.recv().await {
                Some(data) => data,
                None => {
                    warn!("incomming tx is closed");
                    break;
                }
            };
            self.process_packet(client, packet).await;
        }
    }
    #[instrument(name = "Dispatcher::run", skip(self))]
    async fn run(self) {
        let shutdown = self.shutdown.clone();
        tokio::select! {
            _ = shutdown.notified() => (),
            _ = self.process_forever() => (),
        }
    }
    pub async fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }
}
