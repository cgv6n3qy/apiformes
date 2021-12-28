mod cfg;
pub mod clients;
mod config;
mod dispatcher;
pub mod error;
mod topics;

use crate::packets::prelude::Packet;
use clients::{Client, ClientManager};
pub use config::{MqttServerConfig, Permeability};
use dispatcher::Dispatcher;
use error::ServerError;
use std::mem::size_of;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{mpsc::channel, Notify, RwLock},
    task::JoinHandle,
};
use topics::Topics;
use tracing::{error, info, instrument};
pub struct MqttServer {
    clients: Arc<RwLock<HashMap<String, Client>>>,
    shutdown: Arc<Notify>,
    workers: Vec<JoinHandle<()>>,
    cfg: Arc<MqttServerConfig>,
    topics: Arc<RwLock<Topics>>,
}

impl MqttServer {
    #[instrument(name = "MqttServer::new", skip(cfg))]
    pub async fn new(cfg: MqttServerConfig) -> Result<Self, ServerError> {
        let queue_len = cfg.dispatcher_queue_size / size_of::<(String, Packet)>();
        let (incoming_tx, incoming_rx) = channel(queue_len);
        let shutdown = Arc::new(Notify::new());
        let cfg = Arc::new(cfg);
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let mut workers =
            ClientManager::start(cfg.clone(), clients.clone(), shutdown.clone(), incoming_tx)
                .await?;
        let topics = Arc::new(RwLock::new(Topics::new()));
        let dispatcher = Dispatcher::new(
            topics.clone(),
            cfg.clone(),
            shutdown.clone(),
            clients.clone(),
            incoming_rx,
        );
        workers.push(dispatcher.spawn().await);
        Ok(MqttServer {
            clients,
            shutdown,
            workers,
            cfg,
            topics,
        })
    }

    #[instrument(name = "MqttServer::shutdown", skip(self))]
    pub async fn shutdown(self) {
        // TODO keep track of https://github.com/tokio-rs/tokio/issues/3903
        self.shutdown.notify_one();
        for worker in self.workers {
            if let Err(e) = worker.await {
                error!("Failed killing one of the workers, {:?}", e);
            };
        }
        info!("Shutting down");
    }
    pub fn get_topics(&self) -> &RwLock<Topics> {
        &self.topics
    }
    pub async fn clients(&self) -> Vec<String> {
        self.clients
            .read()
            .await
            .keys()
            .map(|x| x.to_owned())
            .collect()
    }
    pub fn config(&self) -> &MqttServerConfig {
        &self.cfg
    }
}
