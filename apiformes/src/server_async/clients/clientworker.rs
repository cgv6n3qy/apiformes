use super::{mqttclient::MqttClient, noiseclient::NoiseClient, Client};
use crate::packets::prelude::*;
use crate::server_async::{
    cfg::*, config::MqttServerConfig, error::ServerError, packetinfo::PacketInfo,
};
use std::sync::Arc;
use tokio::sync::{
    mpsc::{unbounded_channel, Sender, UnboundedReceiver},
    Notify,
};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

pub(super) enum Connection {
    Mqtt(MqttClient),
    Noise(Box<NoiseClient>),
}

impl Connection {
    pub async fn recv(&mut self) -> Result<Packet, ServerError> {
        match self {
            Connection::Mqtt(c) => c.recv().await,
            Connection::Noise(n) => n.recv().await,
        }
    }
    pub async fn send(&mut self, p: &Packet) -> Result<(), ServerError> {
        match self {
            Connection::Mqtt(c) => c.send(p).await,
            Connection::Noise(n) => n.send(p).await,
        }
    }
    pub fn is_encrypted(&self) -> bool {
        match self {
            Connection::Mqtt(_) => false,
            Connection::Noise(_) => true,
        }
    }
}

pub(super) struct ClientWorker {
    incoming: Sender<PacketInfo>,
    outgoing: UnboundedReceiver<Packet>,
    conn: Connection,
    cfg: Arc<MqttServerConfig>,
    internals: Client,
}

impl ClientWorker {
    async fn listen(&mut self) -> Result<(), ServerError> {
        tokio::select! {
            p = self.conn.recv() => {
                let packet = p?;
                let p = PacketInfo {
                    senderid: self.internals.clientid.clone(),
                    packet,
                };
                self.incoming.send(p).await.
                    map_err(|_| ServerError::Misc("Error sending incoming packet to processing queue".to_owned()))?;
            }
            p = self.outgoing.recv() => {
                let packet = p.map(Ok).unwrap_or_else(|| Err(ServerError::Misc("outgoing queue lost all its senders".to_owned())))?;
                self.conn.send(&packet).await?;
            }
        }
        Ok(())
    }
    async fn listen_forever(&mut self) {
        loop {
            if let Err(e) = self.listen().await {
                error!(
                    clientid = &*self.internals.clientid,
                    "Received error while listening, {:?}", e
                );
                break;
            }
        }
    }
    #[instrument(name = "ClientWorker::run", skip_all)]
    pub(super) async fn run(mut self) -> String {
        let shutdown = self.internals.shutdown.clone();
        let killme = self.internals.killme.clone();
        tokio::select! {
            _ = killme.notified() => (),
            _ = shutdown.notified() => (),
            _ = self.listen_forever() => (),
        }
        self.internals.clientid
    }

    pub(super) fn internals(&self) -> &Client {
        &self.internals
    }
    pub(super) fn cfg(&self) -> Arc<MqttServerConfig> {
        self.cfg.clone()
    }
    pub(super) fn new(
        c: Connection,
        cfg: Arc<MqttServerConfig>,
        shutdown: Arc<Notify>,
        incoming: Sender<PacketInfo>,
    ) -> Self {
        let (outgoing_tx, outgoing_rx) = unbounded_channel();
        ClientWorker {
            internals: Client::new(shutdown, outgoing_tx, c.is_encrypted(), cfg.max_packet_size),
            incoming,
            outgoing: outgoing_rx,
            conn: c,
            cfg,
        }
    }

    async fn unimplemented(&mut self) -> Result<(), ServerError> {
        let mut connack = ConnAck::new();
        connack.set_reason_code(ConnAckReasonCode::ImplementationSpecificError);
        self.conn.send(&connack.build()).await?;
        Err(ServerError::Misc("Unimplemented".to_owned()))
    }
    #[instrument(name = "ClientWorker::process_connect", skip_all)]
    async fn process_connect(&mut self, connect: Connect) -> Result<(), ServerError> {
        if connect.username().is_some() {
            error!("Client attempted using username for authentication which is not supported");
            return self.unimplemented().await;
        }
        if connect.password().is_some() {
            error!("Client attempted using password for authentication which is not supported");
            return self.unimplemented().await;
        }
        if connect.will().is_some() {
            error!("Client attempted having a will which is not supported");
            return self.unimplemented().await;
        }
        if !connect.flags().contains(ConnectFlags::CLEAN_START) {
            error!("Client attempted reusing session which is not supported");
            return self.unimplemented().await;
        }
        for (k, v) in connect.props_iter() {
            match k {
                Property::SessionExpiryInterval => {
                    self.internals.session_expirary = v.into_u32().unwrap()
                }
                Property::ReceiveMaximum => self.internals.recv_max = v.into_u16().unwrap(),
                Property::MaximumPacketSize => {
                    self.internals.max_packet_size =
                        self.internals.max_packet_size.min(v.into_u32().unwrap())
                }
                Property::TopicAliasMaximum => {
                    self.internals.topic_alias_max = v.into_u16().unwrap()
                }
                Property::RequestResponseInformation => {
                    self.internals.response_info = v.into_bool().unwrap()
                }
                Property::RequestProblemInformation => {
                    self.internals.problem_info = v.into_bool().unwrap()
                }
                Property::UserProperty => warn!(
                    "Client is using strange property in connect packet {:?}",
                    v.into_str_pair().unwrap()
                ),
                Property::AuthenticationMethod => {
                    error!("Client attempted Authentication which is not supported");
                    return self.unimplemented().await;
                }
                Property::AuthenticationData => {
                    error!("Client attempted Authentication which is not supported");
                    return self.unimplemented().await;
                }
                _ => error!(
                    "Internal Error: {:?} should not be part of Connect packet",
                    k
                ),
            }
        }
        let mut connack = ConnAck::new();
        connack.set_reason_code(ConnAckReasonCode::Success);
        // TODO once we support sessions
        // If the Server accepts a connection with Clean Start set to 1, the Server MUST set Session Present to 0 in
        // the CONNACK packet in addition to setting a 0x00 (Success) Reason Code in the CONNACK packet
        // If the Server accepts a connection with Clean Start set to 0 and the Server has Session State for the
        // ClientID, it MUST set Session Present to 1 in the CONNACK packet, otherwise it MUST set Session
        // Present to 0 in the CONNACK packet. In both cases it MUST set a 0x00 (Success) Reason Code in the
        // CONNACK packet
        connack
            .add_prop(
                Property::SessionExpiryInterval,
                MqttPropValue::new_u32(self.internals.session_expirary),
            )
            .unwrap();
        connack
            .add_prop(
                Property::ReceiveMaximum,
                MqttPropValue::new_u16(self.internals.recv_max),
            )
            .unwrap();
        connack
            .add_prop(Property::MaximumQoS, MqttPropValue::new_u8(MAX_QOS))
            .unwrap();
        match connect.clientid() {
            "" => {
                self.internals.clientid = Uuid::new_v4().to_hyphenated().to_string();
                info!("Assigning {} to client", self.internals.clientid);
                connack
                    .add_prop(
                        Property::AssignedClientIdentifier,
                        MqttPropValue::new_string(&self.internals.clientid).unwrap(),
                    )
                    .unwrap();
            }
            x => self.internals.clientid = x.to_string(),
        }
        connack
            .add_prop(
                Property::TopicAliasMaximum,
                MqttPropValue::new_u16(TOPIC_ALIAS_MAX),
            )
            .unwrap();
        connack
            .add_prop(
                Property::WildcardSubscriptionAvailable,
                MqttPropValue::new_bool(WILDCARD_SUB),
            )
            .unwrap();
        connack
            .add_prop(
                Property::SubscriptionIdentifierAvailable,
                MqttPropValue::new_bool(SUB_ID),
            )
            .unwrap();
        connack
            .add_prop(
                Property::SharedSubscriptionAvailable,
                MqttPropValue::new_bool(SHARED_SUB),
            )
            .unwrap();
        connack
            .add_prop(
                Property::ServerKeepAlive,
                MqttPropValue::new_u16(self.cfg.keep_alive),
            )
            .unwrap();
        self.conn.send(&connack.build()).await
    }
    pub async fn connect(&mut self) -> Result<(), ServerError> {
        match self.conn.recv().await? {
            Packet::Connect(c) => self.process_connect(c).await,
            _ => Err(ServerError::FirstPacketNotConnect),
        }
    }
}
