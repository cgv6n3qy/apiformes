mod cfg;
pub mod clients;
mod config;
pub mod error;

use clients::{Client, ClientManager};
pub use config::MqttServerConfig;
use error::ServerError;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{mpsc::unbounded_channel, Notify, RwLock},
    task::JoinHandle,
};
use tracing::{error, info, instrument};

pub struct MqttServer {
    clients: Arc<RwLock<HashMap<String, Client>>>,
    shutdown: Arc<Notify>,
    workers: Vec<JoinHandle<()>>,
    cfg: Arc<MqttServerConfig>,
}

impl MqttServer {
    #[instrument(name = "MqttServer::new", skip(cfg))]
    pub async fn new(cfg: MqttServerConfig) -> Result<Self, ServerError> {
        let (incoming_tx, incoming_rx) = unbounded_channel();
        let shutdown = Arc::new(Notify::new());
        let cfg = Arc::new(cfg);
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let workers =
            ClientManager::start(cfg.clone(), clients.clone(), shutdown.clone(), incoming_tx)
                .await?;
        Ok(MqttServer {
            clients,
            shutdown,
            workers,
            cfg,
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
}
