mod cfg;
pub mod client;
mod config;
pub mod error;

use client::{Client, MqttListener};
pub use config::MqttServerConfig;
use error::ServerError;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
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
            let handle = MqttServer::incomming_mqtt_listener(
                &saddr,
                tx.clone(),
                shutdown.clone(),
                cfg.clone(),
            )
            .await?;
            workers.push(handle)
        };
        Ok(MqttServer {
            new_clients: rx,
            shutdown,
            workers,
            cfg,
        })
    }

    async fn incomming_mqtt_listener(
        saddr: &SocketAddr,
        tx: UnboundedSender<Client>,
        shutdown: Arc<Notify>,
        cfg: Arc<MqttServerConfig>,
    ) -> Result<JoinHandle<()>, ServerError> {
        let listener = TcpListener::bind(saddr).await?;
        info!(
            SocketAddr = &*format!("{}", saddr),
            "Starting Listener for incoming unencrypted connections"
        );

        Ok(tokio::spawn(async move {
            MqttListener::new(listener, tx, shutdown, cfg).run().await
        }))
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
