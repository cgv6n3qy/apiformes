mod client;
mod clientworker;
mod mqttclient;
pub use client::Client;
use clientworker::ClientWorker;
pub use mqttclient::MqttListener;

use crate::server_async::{config::MqttServerConfig, error::ServerError};
use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Notify, RwLock,
    },
    task::JoinHandle,
};
use tracing::{warn, info, instrument};

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
    ) -> Result<Vec<JoinHandle<()>>, ServerError> {
        let (tx, rx) = unbounded_channel();

        let mut workers = Vec::new();
        if let Some(saddr) = cfg.mqtt_socketaddr {
            let handle = ClientManager::incomming_mqtt_listener(
                &saddr,
                tx.clone(),
                shutdown.clone(),
                cfg.clone(),
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
            //TODO 1 handle clients takeover
            //TODO 2 spawn worker
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
    ) -> Result<JoinHandle<()>, ServerError> {
        let listener = TcpListener::bind(saddr).await?;
        info!(
            SocketAddr = &*format!("{}", saddr),
            "Starting listener for incoming unencrypted connections"
        );

        Ok(tokio::spawn(async move {
            MqttListener::new(listener, tx, shutdown, cfg).run().await
        }))
    }
}
