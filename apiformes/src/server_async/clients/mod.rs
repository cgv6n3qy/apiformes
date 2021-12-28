mod client;
mod clientworker;
mod mqttclient;
#[cfg(feature = "noise")]
mod noiseclient;

use crate::packets::prelude::Packet;
use crate::server_async::{config::MqttServerConfig, error::ServerError};
pub use client::Client;
use clientworker::ClientWorker;
pub use mqttclient::MqttListener;
pub use noiseclient::NoiseListener;
use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
        Notify, RwLock,
    },
    task::JoinHandle,
};
use tracing::{info, instrument, warn};

pub struct ClientManager {
    rx: UnboundedReceiver<ClientWorker>,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    cfg: Arc<MqttServerConfig>,
    shutdown: Arc<Notify>,
}

impl ClientManager {
    fn new(
        cfg: Arc<MqttServerConfig>,
        clients: Arc<RwLock<HashMap<String, Client>>>,
        shutdown: Arc<Notify>,
        rx: UnboundedReceiver<ClientWorker>,
    ) -> Self {
        ClientManager {
            rx,
            clients,
            cfg,
            shutdown,
        }
    }
    #[instrument(name = "ClientManager::start", skip_all)]
    pub async fn start(
        cfg: Arc<MqttServerConfig>,
        clients: Arc<RwLock<HashMap<String, Client>>>,
        shutdown: Arc<Notify>,
        incoming: Sender<(String, Packet)>,
    ) -> Result<Vec<JoinHandle<()>>, ServerError> {
        let (tx, rx) = unbounded_channel();

        let mut workers = Vec::new();
        if let Some(saddr) = cfg.mqtt_socketaddr {
            let handle = ClientManager::incomming_mqtt_listener(
                &saddr,
                tx.clone(),
                shutdown.clone(),
                cfg.clone(),
                incoming.clone(),
            )
            .await?;
            workers.push(handle)
        }

        #[cfg(feature = "noise")]
        if let Some(saddr) = cfg.noise_socketaddr {
            let handle = ClientManager::incomming_noise_listener(
                &saddr,
                tx.clone(),
                shutdown.clone(),
                cfg.clone(),
                incoming,
            )
            .await?;
            workers.push(handle)
        }

        let man = ClientManager::new(cfg, clients, shutdown, rx);
        workers.push(man.start_processing().await);
        Ok(workers)
    }

    #[instrument(name = "ClientManager::process_forever", skip_all)]
    async fn process_forever(mut self) {
        loop {
            let worker = match self.rx.recv().await {
                Some(worker) => worker,
                None => {
                    warn!("All ClientWorker Tx halves has been dropped which is means server is shutting down or this is an internal bug,");
                    return;
                }
            };
            let client = worker.internals();
            self.clients
                .write()
                .await
                .insert(client.clientid.to_owned(), client.clone());
            //TODO handle clients takeover
            tokio::spawn(async move { worker.run().await });
        }
    }
    async fn run(self) {
        let shutdown = self.shutdown.clone();
        tokio::select! {
            _ = shutdown.notified() => (),
            _ = self.process_forever() => ()
        };
    }
    async fn start_processing(self) -> JoinHandle<()> {
        info!("Starting clients manager");
        tokio::spawn(async move { self.run().await })
    }
    async fn incomming_mqtt_listener(
        saddr: &SocketAddr,
        tx: UnboundedSender<ClientWorker>,
        shutdown: Arc<Notify>,
        cfg: Arc<MqttServerConfig>,
        incoming: Sender<(String, Packet)>,
    ) -> Result<JoinHandle<()>, ServerError> {
        let listener = TcpListener::bind(saddr).await?;
        info!(
            SocketAddr = &*format!("{}", saddr),
            "Starting listener for incoming unencrypted connections"
        );

        Ok(tokio::spawn(async move {
            MqttListener::new(listener, tx, shutdown, cfg, incoming)
                .run()
                .await
        }))
    }

    async fn incomming_noise_listener(
        saddr: &SocketAddr,
        tx: UnboundedSender<ClientWorker>,
        shutdown: Arc<Notify>,
        cfg: Arc<MqttServerConfig>,
        incoming: Sender<(String, Packet)>,
    ) -> Result<JoinHandle<()>, ServerError> {
        let listener = TcpListener::bind(saddr).await?;
        info!(
            SocketAddr = &*format!("{}", saddr),
            "Starting listener for incoming encrypted connections"
        );

        Ok(tokio::spawn(async move {
            NoiseListener::new(listener, tx, shutdown, cfg, incoming)
                .run()
                .await
        }))
    }
}
