use super::clientworker::{ClientWorker, Connection};
use crate::server_async::{config::MqttServerConfig, error::ServerError, packetinfo::PacketInfo};
use apiformes_packet::prelude::*;
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use std::{fmt, net::SocketAddr, sync::Arc};
use tokio::time::{sleep, Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, Take},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    sync::{
        mpsc::{Sender, UnboundedSender},
        Notify,
    },
};
use tracing::{error, info, instrument, warn};

pub struct MqttClient {
    tcp_reader: Take<OwnedReadHalf>,
    tcp_writer: OwnedWriteHalf,
    bytes: BytesMut,
    saddr: SocketAddr,
    max_packet_size: u32,
}

impl fmt::Debug for MqttClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MqttWorkerClient({:?})", self.saddr)
    }
}

impl MqttClient {
    pub fn new(stream: TcpStream, saddr: SocketAddr, max_packet_size: u32) -> Self {
        let (tcp_reader, tcp_writer) = stream.into_split();

        MqttClient {
            tcp_reader: tcp_reader.take(max_packet_size as u64),
            tcp_writer,
            saddr,
            bytes: BytesMut::with_capacity(max_packet_size as usize),
            max_packet_size,
        }
    }
    pub async fn recv(&mut self) -> Result<Packet, ServerError> {
        loop {
            let mut cursor = Cursor::new(&self.bytes[..]);
            match Packet::from_bytes(&mut cursor) {
                Ok(packet) => {
                    self.bytes.advance(packet.frame_len());
                    return Ok(packet);
                }
                Err(DataParseError::InsufficientBuffer {
                    needed: _,
                    available: _,
                }) => {
                    if self.bytes.remaining() == self.max_packet_size as usize {
                        return Err(ServerError::MaxPacketSizeExceeded);
                    }
                    self.tcp_reader
                        .set_limit(self.max_packet_size as u64 - self.bytes.remaining() as u64);
                    self.tcp_reader.read_buf(&mut self.bytes).await?;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    pub async fn send(&mut self, p: &Packet) -> Result<(), ServerError> {
        let mut bytes = BytesMut::with_capacity(p.frame_len());
        p.to_bytes(&mut bytes)?;
        self.tcp_writer.write_all_buf(&mut bytes).await?;
        Ok(())
    }
}

pub struct MqttListener {
    mqtt_listener: TcpListener,
    queue: UnboundedSender<ClientWorker>,
    shutdown: Arc<Notify>,
    cfg: Arc<MqttServerConfig>,
    incoming: Sender<PacketInfo>,
}

impl MqttListener {
    pub(super) fn new(
        listener: TcpListener,
        queue: UnboundedSender<ClientWorker>,
        shutdown: Arc<Notify>,
        cfg: Arc<MqttServerConfig>,
        incoming: Sender<PacketInfo>,
    ) -> MqttListener {
        MqttListener {
            mqtt_listener: listener,
            queue,
            shutdown,
            cfg,
            incoming,
        }
    }
    async fn listen(&mut self) -> Result<(), ServerError> {
        let (stream, saddr) = self.mqtt_listener.accept().await?;
        let connection = Connection::Mqtt(MqttClient::new(stream, saddr, self.cfg.max_packet_size));
        let client = ClientWorker::new(
            connection,
            self.cfg.clone(),
            self.shutdown.clone(),
            self.incoming.clone(),
        );
        connect_client(client, saddr, self.queue.clone(), self.shutdown.clone());
        Ok(())
    }
    #[instrument(name = "MqttListener::listen_forever", skip_all)]
    async fn listen_forever(&mut self) -> ! {
        loop {
            if let Err(e) = self.listen().await {
                error!("Error listening to new connections, {:?}", e);
            }
        }
    }
    #[instrument(name = "MqttListener::run", skip_all)]
    pub async fn run(mut self) {
        let shutdown = self.shutdown.clone();
        tokio::select! {
            _ = shutdown.notified() => (),
            _ = self.listen_forever() => ()
        };
        info!("shutting down");
    }
}

fn connect_client(
    client: ClientWorker,
    saddr: SocketAddr,
    queue: UnboundedSender<ClientWorker>,
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
    mut client: ClientWorker,
    saddr: SocketAddr,
    queue: UnboundedSender<ClientWorker>,
    shutdown: Arc<Notify>,
) {
    let keep_alive = client.cfg().keep_alive as u64;

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
