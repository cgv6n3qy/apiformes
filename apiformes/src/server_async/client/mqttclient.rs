use super::{Client, Connection};
use crate::packets::prelude::*;
use crate::server_async::{config::MqttServerConfig, error::ServerError};
use bytes::{Buf, BytesMut};
use std::{fmt, net::SocketAddr, sync::Arc};
use tokio::time::{sleep, Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc::UnboundedSender, Notify},
};
use tracing::{error, info, instrument, warn};

pub struct MqttClient {
    stream: TcpStream,
    bytes: BytesMut,
    saddr: SocketAddr,
}

impl fmt::Debug for MqttClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MqttClient({:?})", self.saddr)
    }
}

impl MqttClient {
    pub fn new(stream: TcpStream, saddr: SocketAddr) -> Self {
        MqttClient {
            stream,
            saddr,
            bytes: BytesMut::new(),
        }
    }
    pub async fn recv(&mut self) -> Result<Packet, ServerError> {
        self.stream.read_buf(&mut self.bytes).await?;
        loop {
            match Packet::from_bytes(&mut self.bytes.clone()) {
                Ok(packet) => {
                    self.bytes.advance(packet.frame_len());
                    return Ok(packet);
                }
                //TODO this may be optimized to read once
                Err(DataParseError::InsufficientBuffer {
                    needed: _,
                    available: _,
                }) => {
                    self.stream.read_buf(&mut self.bytes).await?;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    pub async fn send(&mut self, p: &Packet) -> Result<(), ServerError> {
        let mut bytes = BytesMut::with_capacity(p.frame_len());
        p.to_bytes(&mut bytes)?;
        self.stream.write_buf(&mut bytes).await?;
        Ok(())
    }
}

pub struct MqttListener {
    mqtt_listener: TcpListener,
    queue: UnboundedSender<Client>,
    shutdown: Arc<Notify>,
    cfg: Arc<MqttServerConfig>,
}

impl MqttListener {
    pub fn new(
        listener: TcpListener,
        queue: UnboundedSender<Client>,
        shutdown: Arc<Notify>,
        cfg: Arc<MqttServerConfig>,
    ) -> MqttListener {
        MqttListener {
            mqtt_listener: listener,
            queue,
            shutdown,
            cfg,
        }
    }
    async fn listen(&mut self) -> Result<(), ServerError> {
        let (stream, saddr) = self.mqtt_listener.accept().await?;
        let connection = Connection::Mqtt(MqttClient::new(stream, saddr));
        let client = Client::new(connection, self.cfg.clone());
        connect_client(
            client,
            saddr.clone(),
            self.queue.clone(),
            self.shutdown.clone(),
        );
        Ok(())
    }
    #[instrument(skip(self))]
    async fn listen_forever(&mut self) -> ! {
        loop {
            if let Err(e) = self.listen().await {
                error!("Error listening to new connections, {:?}", e);
            }
        }
    }
    pub async fn run(mut self) {
        let shutdown = self.shutdown.clone();
        tokio::select! {
            _ = shutdown.notified() => (),
            _ = self.listen_forever() => ()
        };
    }
}

fn connect_client(
    client: Client,
    saddr: SocketAddr,
    queue: UnboundedSender<Client>,
    shutdown: Arc<Notify>,
) {
    tokio::spawn(async move { _connect_client(client, saddr, queue, shutdown).await });
}

enum ConnectState {
    Err(ServerError),
    Success,
    ShuttingDown,
}

impl From<Result<(), ServerError>> for ConnectState {
    fn from(v: Result<(), ServerError>) -> ConnectState {
        match v {
            Ok(_) => ConnectState::Success,
            Err(e) => ConnectState::Err(e),
        }
    }
}

async fn _connect_client(
    mut client: Client,
    saddr: SocketAddr,
    queue: UnboundedSender<Client>,
    shutdown: Arc<Notify>,
) {
    let keep_alive = client.cfg.keep_alive as u64;

    let state = tokio::select! {
        _ = shutdown.notified() => ConnectState::ShuttingDown,
        v = client.connect() => v.into(),
        _ = sleep(Duration::new(keep_alive, 0)) => ConnectState::Err(ServerError::Misc("TimeOut".to_string())),
    };
    let saddr = format!("{}", saddr);
    match state {
        ConnectState::Success => info!(SocketAddr = &*saddr, "MQTT Connection established"),
        ConnectState::ShuttingDown => info!(SocketAddr = &*saddr, "Shutting down"),
        ConnectState::Err(e) => {
            warn!(
                SocketAddr = &*saddr,
                " Failed to establish MQTT connection, {:?}", e
            );
            return;
        }
    }
    if queue.send(client).is_err() {
        error!("MPSC channel for new connections is broken");
    }
}
