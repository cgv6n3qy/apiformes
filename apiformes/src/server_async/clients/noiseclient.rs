use super::clientworker::{ClientWorker, Connection};
use crate::packets::prelude::*;
use crate::server_async::{cfg::NOISE_PATTERN, config::MqttServerConfig, error::ServerError};
use bytes::{Buf, Bytes, BytesMut};
use snow::{HandshakeState, TransportState};
use std::{fmt, net::SocketAddr, sync::Arc};
use tokio::time::{sleep, Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc::UnboundedSender, Notify},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::{error, info, instrument, warn};

use futures::{SinkExt, StreamExt};
use tracing::trace;
pub struct NoiseClient {
    stream: Framed<TcpStream, LengthDelimitedCodec>,
    saddr: SocketAddr,
    crypto: TransportState,
}

impl fmt::Debug for NoiseClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NoiseWorkerClient({:?})", self.saddr)
    }
}

impl NoiseClient {
    pub fn new(
        stream: Framed<TcpStream, LengthDelimitedCodec>,
        saddr: SocketAddr,
        crypto: TransportState,
    ) -> Self {
        NoiseClient {
            stream,
            saddr,
            crypto,
        }
    }
    pub async fn recv(&mut self) -> Result<Packet, ServerError> {
        //let frame = self.stream.send();
        let frame = self
            .stream
            .next()
            .await
            .ok_or_else(||ServerError::Misc("Client disconnected".to_owned()))??;
        //TODO when you implement noise protocol by hand... make sure to make this happen in place
        let mut message = vec![0; frame.remaining()];
        self.crypto.read_message(&frame[..], &mut message)?;

        let mut buf: &[u8] = message.as_ref();
        match Packet::from_bytes(&mut buf) {
            Ok(packet) => Ok(packet),
            Err(DataParseError::InsufficientBuffer {
                needed: _,
                available: _,
            }) => Err(DataParseError::BadPacketType.into()),
            Err(e) => Err(e.into()),
        }
    }
    pub async fn send(&mut self, p: &Packet) -> Result<(), ServerError> {
        let mut bytes = BytesMut::with_capacity(p.frame_len());
        p.to_bytes(&mut bytes)?;
        let mut frame = vec![0; bytes.remaining() + 100];
        let size = self.crypto.write_message(&bytes[..], &mut frame)?;
        self.stream
            .send(Bytes::copy_from_slice(&frame[..size]))
            .await?;
        Ok(())
    }
}

pub struct NoiseListener {
    listener: TcpListener,
    queue: UnboundedSender<ClientWorker>,
    shutdown: Arc<Notify>,
    cfg: Arc<MqttServerConfig>,
    incoming: UnboundedSender<(String, Packet)>,
}

impl NoiseListener {
    pub(super) fn new(
        listener: TcpListener,
        queue: UnboundedSender<ClientWorker>,
        shutdown: Arc<Notify>,
        cfg: Arc<MqttServerConfig>,
        incoming: UnboundedSender<(String, Packet)>,
    ) -> NoiseListener {
        NoiseListener {
            listener,
            queue,
            shutdown,
            cfg,
            incoming,
        }
    }
    async fn listen(&mut self) -> Result<(), ServerError> {
        let (stream, saddr) = self.listener.accept().await?;
        connect_client(
            stream,
            saddr,
            self.queue.clone(),
            self.shutdown.clone(),
            self.cfg.clone(),
            self.incoming.clone(),
        );
        Ok(())
    }
    #[instrument(name = "NoiseListener::listen_forever", skip_all)]
    async fn listen_forever(&mut self) -> ! {
        loop {
            if let Err(e) = self.listen().await {
                error!("Error listening to new connections, {:?}", e);
            }
        }
    }
    #[instrument(name = "NoiseListener::run", skip_all)]
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
    stream: TcpStream,
    saddr: SocketAddr,
    queue: UnboundedSender<ClientWorker>,
    shutdown: Arc<Notify>,
    cfg: Arc<MqttServerConfig>,
    incoming: UnboundedSender<(String, Packet)>,
) {
    tokio::spawn(
        async move { _connect_client(stream, saddr, queue, shutdown, cfg, incoming).await },
    );
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
    stream: TcpStream,
    saddr: SocketAddr,
    queue: UnboundedSender<ClientWorker>,
    shutdown: Arc<Notify>,
    cfg: Arc<MqttServerConfig>,
    incoming: UnboundedSender<(String, Packet)>,
) {
    let keep_alive = cfg.keep_alive as u64;
    let mut stream = Framed::new(stream, LengthDelimitedCodec::new());

    let mut responder = snow::Builder::new(NOISE_PATTERN.parse().unwrap())
        .local_private_key(&cfg.private_key[..])
        .build_responder()
        .unwrap();

    let state = tokio::select! {
        _ = shutdown.notified() => ConnectState::ShuttingDown,
        v = handshake(&mut stream, &mut responder) => v.into(),
        _ = sleep(Duration::new(keep_alive * 3, 0)) => ConnectState::Err(ServerError::Misc("TimeOut".to_string())),
    };
    let saddr_str = format!("{}", saddr);
    match state {
        ConnectState::Success => info!(SocketAddr = &*saddr_str, "MQTT Connection established"),
        ConnectState::ShuttingDown => info!(SocketAddr = &*saddr_str, "Shutting down"),
        ConnectState::Err(e) => {
            warn!(
                SocketAddr = &*saddr_str,
                " Failed to establish Noise handshake, {:?}", e
            );
            return;
        }
    }
    let transport = responder.into_transport_mode().unwrap();

    let nc = NoiseClient::new(stream, saddr, transport);
    let mut client = ClientWorker::new(Connection::Noise(Box::new(nc)), cfg, shutdown.clone(), incoming);
    let state = tokio::select! {
        _ = shutdown.notified() => ConnectState::ShuttingDown,
        v = client.connect() => v.into(),
        _ = sleep(Duration::new(keep_alive, 0)) => ConnectState::Err(ServerError::Misc("TimeOut".to_string())),
    };
    match state {
        ConnectState::Success => info!(SocketAddr = &*saddr_str, "MQTT Connection established"),
        ConnectState::ShuttingDown => info!(SocketAddr = &*saddr_str, "Shutting down"),
        ConnectState::Err(e) => {
            warn!(
                SocketAddr = &*saddr_str,
                " Failed to establish MQTT connection, {:?}", e
            );
            return;
        }
    }
    if queue.send(client).is_err() {
        error!("MPSC channel for new connections is broken");
    }
}

async fn handshake(
    stream: &mut Framed<TcpStream, LengthDelimitedCodec>,
    handshake: &mut HandshakeState,
) -> Result<(), ServerError> {
    //  -> e, es
    let frame = stream
        .next()
        .await
        .ok_or_else(|| ServerError::Misc("Client disconnected".to_owned()))??;
    trace!("-> e, es");
    trace!("{:x?}", &frame[..]);
    let mut out_buf = [0; 200];
    handshake.read_message(&frame[..], &mut [])?;

    // <- e, ee
    let size = handshake.write_message(&[], &mut out_buf)?;
    trace!("<- e, ee");
    trace!("{:x?}", &out_buf[..size]);
    stream
        .send(Bytes::copy_from_slice(&out_buf[..size]))
        .await?;

    // -> s, se
    let frame = stream
        .next()
        .await
        .ok_or_else(|| ServerError::Misc("Client disconnected".to_owned()))??;
    trace!("-> s, se");
    trace!("{:x?}", &frame[..]);
    handshake.read_message(&frame[..], &mut [])?;

    Ok(())
}
