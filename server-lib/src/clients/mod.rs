mod client;
mod clientworker;
mod mqttclient;
#[cfg(feature = "noise")]
mod noiseclient;

use crate::{config::MqttServerConfig, error::ServerError, packetinfo::PacketInfo};
pub use client::Client;
use clientworker::ClientWorker;
use futures::{stream::FuturesUnordered, StreamExt};
pub use mqttclient::MqttListener;
#[cfg(feature = "noise")]
pub use noiseclient::NoiseListener;
use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
        Notify, RwLock,
    },
    task::{JoinError, JoinHandle},
};
use tracing::{error, info, instrument, warn};

pub struct ClientManager {
    rx: UnboundedReceiver<ClientWorker>,
    clients: Arc<RwLock<HashMap<Arc<str>, Client>>>,
    cfg: Arc<MqttServerConfig>,
    shutdown: Arc<Notify>,
    workers: FuturesUnordered<JoinHandle<Arc<str>>>,
}

impl ClientManager {
    fn new(
        cfg: Arc<MqttServerConfig>,
        clients: Arc<RwLock<HashMap<Arc<str>, Client>>>,
        shutdown: Arc<Notify>,
        rx: UnboundedReceiver<ClientWorker>,
    ) -> Self {
        ClientManager {
            rx,
            clients,
            cfg,
            shutdown,
            workers: FuturesUnordered::new(),
        }
    }
    #[instrument(name = "ClientManager::start", skip_all)]
    pub async fn start(
        cfg: Arc<MqttServerConfig>,
        clients: Arc<RwLock<HashMap<Arc<str>, Client>>>,
        shutdown: Arc<Notify>,
        incoming: Sender<PacketInfo>,
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

    async fn process_new_worker(&mut self, maybe_worker: Option<ClientWorker>) -> bool {
        let worker = match maybe_worker {
            Some(worker) => worker,
            None => {
                warn!("All ClientWorker Tx halves has been dropped which is means server is shutting down or this is an internal bug,");
                return false;
            }
        };
        let client = worker.internals();
        self.clients
            .write()
            .await
            .insert(client.clientid.clone(), client.clone());
        self.workers
            .push(tokio::spawn(async move { worker.run().await }));
        true
    }
    async fn process_retiring_worker(&mut self, maybe_id: Option<Result<Arc<str>, JoinError>>) {
        //TODO we don't remove the worker information from the topic file system yet
        match maybe_id {
            Some(Err(e)) => error!(
                "Failed joining one of the threads, possible orphan threads running, {:?}",
                e
            ),
            Some(Ok(id)) => {
                self.clients.write().await.remove(&id);
            }
            None => (),
        };
    }
    #[instrument(name = "ClientManager::process_forever", skip_all)]
    async fn process_forever(&mut self) {
        loop {
            // the reason we have this before the tokio select is because when workers set is empty
            // we will be burning through CPU cycles because we have nothing to await for,
            // so it is better to at least make sure that there is 1 element in the queue before we do anything
            if self.workers.is_empty() {
                let w = self.rx.recv().await;
                if !self.process_new_worker(w).await {
                    break;
                }
            }
            tokio::select! {
                w = self.rx.recv() => if !self.process_new_worker(w).await {
                    break;
                },
                clientid = self.workers.next() => self.process_retiring_worker(clientid).await,
            };
        }
    }
    async fn run(mut self) {
        let shutdown = self.shutdown.clone();
        tokio::select! {
            _ = shutdown.notified() => (),
            _ = self.process_forever() => ()
        };
        while self.workers.next().await.is_some() {
            // wait for all workers to yeield
        }
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
        incoming: Sender<PacketInfo>,
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

    #[cfg(feature = "noise")]
    async fn incomming_noise_listener(
        saddr: &SocketAddr,
        tx: UnboundedSender<ClientWorker>,
        shutdown: Arc<Notify>,
        cfg: Arc<MqttServerConfig>,
        incoming: Sender<PacketInfo>,
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
