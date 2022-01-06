#[cfg(feature = "noise")]
use super::Permeability;
use super::{
    topics::{SubscriptionFlags, TopicsTable},
    Client, MqttServerConfig, ServerError,
};
use tokio::sync::{mpsc::Receiver, Notify, RwLock};
use tokio::task::JoinHandle;

use super::packetinfo::PacketInfo;
use apiformes_packet::prelude::*;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use tracing::{error, instrument, trace, warn};

pub struct Dispatcher {
    topics: Arc<TopicsTable>,
    cfg: Arc<MqttServerConfig>,
    shutdown: Arc<Notify>,
    clients: Arc<RwLock<HashMap<Arc<str>, Client>>>,
    incoming: Receiver<PacketInfo>,
}

impl Dispatcher {
    pub fn new(
        topics: Arc<TopicsTable>,
        cfg: Arc<MqttServerConfig>,
        shutdown: Arc<Notify>,
        clients: Arc<RwLock<HashMap<Arc<str>, Client>>>,
        incoming: Receiver<PacketInfo>,
    ) -> Self {
        Dispatcher {
            topics,
            cfg,
            shutdown,
            clients,
            incoming,
        }
    }
    async fn unimplemented(&mut self, client: &str) -> Result<(), ServerError> {
        let disconnect = Disconnect::new(DisconnectReasonCode::ImplementationSpecificError).build();
        let clients = self.clients.read().await;
        let c = clients.get(client).unwrap();
        if c.send(disconnect).is_err() {
            error!(clientid = client, "Internal Error: tx closed");
        }
        Err(ServerError::Misc("Unimplemented".to_owned()))
    }

    #[instrument(skip_all)]
    async fn process_publish(&mut self, client: &str, publish: Publish) -> Result<(), ServerError> {
        trace!("Processing a publish packet");
        #[cfg(feature = "noise")]
        let strict_encryption = {
            let clients = self.clients.read().await;
            match clients.get(client) {
                Some(c) => c.encrypted() && self.cfg.channel_permeability == Permeability::Strict,
                None => {
                    warn!(
                        clientid = client,
                        "Client Prematurely shutdown before its publish request could be processed"
                    );
                    return Ok(());
                }
            }
        };
        match publish.qos() {
            QoS::QoS0 => (),
            QoS::QoS1 => return self.unimplemented(client).await,
            QoS::QoS2 => return self.unimplemented(client).await,
        }
        if publish
            .flags()
            .intersects(PublishFlags::DUP | PublishFlags::RETAIN)
        {
            return self.unimplemented(client).await;
        }
        let topic = publish.topic_name();
        let mut response = Publish::new(topic.clone(), publish.payload()).unwrap();
        for (k, v) in publish.props_iter() {
            match k {
                Property::PayloadFormatIndicator => response
                    .add_prop(Property::PayloadFormatIndicator, v.clone())
                    .unwrap(),
                Property::MessageExpiryInterval => return self.unimplemented(client).await,
                Property::TopicAlias => return self.unimplemented(client).await,
                Property::ResponseTopic => response
                    .add_prop(Property::ResponseTopic, v.clone())
                    .unwrap(),
                Property::CorrelationData => response
                    .add_prop(Property::CorrelationData, v.clone())
                    .unwrap(),
                Property::UserProperty => response
                    .add_prop(Property::UserProperty, v.clone())
                    .unwrap(),
                Property::SubscriptionIdentifier => return self.unimplemented(client).await,
                Property::ContentType => {
                    response.add_prop(Property::ContentType, v.clone()).unwrap()
                }
                _ => error!(
                    "Internal Error: {:?} should not be part of publish packet",
                    k
                ),
            }
        }
        let resp = response.build();
        let clients = self.clients.read().await;

        for (target, info) in self.topics.get_all_subscribed(topic).await {
            if target.as_ref() == client && info.flags.contains(SubscriptionFlags::NO_LOCAL) {
                continue;
            }
            if info.flags.contains(SubscriptionFlags::RETAIN_AS_PUBLISHED) {
                unimplemented!();
            }
            if let Some(c) = clients.get(&target) {
                #[cfg(feature = "noise")]
                if strict_encryption && !c.encrypted() {
                    continue;
                }
                match info.qos {
                    QoS::QoS0 => {
                        if c.send(resp.clone()).is_err() {
                            trace!(clientid = target.as_ref(), "client shutdown: tx closed");
                        };
                    }
                    _ => unimplemented!(),
                }
            }
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn process_subscribe(
        &mut self,
        client: &Arc<str>,
        sub: Subscribe,
    ) -> Result<(), ServerError> {
        trace!("Processing a subscribe packet");
        let ident = sub.packet_identifier();
        for (k, _) in sub.props_iter() {
            match k {
                Property::SubscriptionIdentifier => return self.unimplemented(client).await,
                Property::UserProperty => {
                    warn!(clientid = client.as_ref(), "Received unknown user property")
                }
                _ => error!(
                    "Internal Error: {:?} should not be part of publish packet",
                    k
                ),
            }
        }
        let mut suback = SubAck::new(ident);
        for (topic, options) in sub.topics_iter() {
            let qos: QoS = (*options).try_into()?;
            let mut flags = SubscriptionFlags::empty();
            match qos {
                QoS::QoS0 => (),
                _ => {
                    suback.add_reason_code(SubAckReasonCode::ImplementationSpecificError);
                    continue;
                }
            }
            match (*options).try_into()? {
                RetainHandling::DoNotSend => (),
                _ => {
                    suback.add_reason_code(SubAckReasonCode::ImplementationSpecificError);
                    continue;
                }
            }
            if options.contains(SubscriptionOptions::NO_LOCAL) {
                flags |= SubscriptionFlags::NO_LOCAL;
            }
            if options.contains(SubscriptionOptions::RETAIN_AS_PUBLISHED) {
                flags |= SubscriptionFlags::RETAIN_AS_PUBLISHED;
            }
            self.topics
                .subscribe(client.clone(), topic.clone(), qos, flags)
                .await;
            match qos {
                QoS::QoS0 => suback.add_reason_code(SubAckReasonCode::GrantedQoS0),
                QoS::QoS1 => suback.add_reason_code(SubAckReasonCode::GrantedQoS1),
                QoS::QoS2 => suback.add_reason_code(SubAckReasonCode::GrantedQoS2),
            }
        }

        let clients = self.clients.read().await;
        if let Some(c) = clients.get(client) {
            if c.send(suback.build()).is_err() {
                error!(clientid = client.as_ref(), "Internal Error: tx closed");
            }
        }
        Ok(())
    }
    async fn process_packet(
        &mut self,
        client: Arc<str>,
        packet: Packet,
    ) -> Result<(), ServerError> {
        match packet {
            Packet::Publish(publish) => self.process_publish(&client, publish).await,
            Packet::Subscribe(sub) => self.process_subscribe(&client, sub).await,
            _ => self.unimplemented(&client).await,
        }
    }
    async fn process_forever(mut self) {
        loop {
            let packetinfo = match self.incoming.recv().await {
                Some(p) => p,
                None => {
                    warn!("incomming tx is closed");
                    break;
                }
            };
            if let Err(e) = self
                .process_packet(packetinfo.senderid.clone(), packetinfo.packet)
                .await
            {
                error!(clientid = &*packetinfo.senderid, "{:?}", e);
            }
        }
    }
    #[instrument(name = "Dispatcher::run", skip(self))]
    async fn run(self) {
        let shutdown = self.shutdown.clone();
        tokio::select! {
            _ = shutdown.notified() => (),
            _ = self.process_forever() => (),
        }
    }
    pub async fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }
}
