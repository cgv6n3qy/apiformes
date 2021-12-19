mod cfg;
pub mod client;
mod config;
pub mod error;

use client::{Client, MqttListener};
pub use config::MqttServerConfig;
use error::ServerError;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver},
        Notify,
    },
    task::JoinHandle,
};
use tracing::{error, info, instrument};

pub struct MqttServer {
    new_clients: UnboundedReceiver<Client>,
    shutdown: Arc<Notify>,
    workers: Vec<JoinHandle<()>>,
    cfg: Arc<MqttServerConfig>,
}

impl MqttServer {
    #[instrument(skip(cfg))]
    pub async fn new(cfg: MqttServerConfig) -> Result<Self, ServerError> {
        let (tx, rx) = unbounded_channel();
        let shutdown = Arc::new(Notify::new());
        let mut workers = Vec::new();
        let cfg = Arc::new(cfg);
        if let Some(saddr) = cfg.mqtt_socketaddr {
            let listener = TcpListener::bind(saddr).await?;
            let tx2 = tx.clone();
            let shutdown2 = shutdown.clone();
            let cfg2 = cfg.clone();
            info!(
                SocketAddr = &*format!("{}", saddr),
                "Starting MQTT listener"
            );
            let handle = tokio::spawn(async move {
                MqttListener::new(listener, tx2, shutdown2, cfg2)
                    .run()
                    .await
            });
            workers.push(handle)
        };
        Ok(MqttServer {
            new_clients: rx,
            shutdown,
            workers,
            cfg,
        })
    }
    #[instrument(skip(self))]
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
