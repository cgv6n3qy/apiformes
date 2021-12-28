use super::{topics::Topics, Client, MqttServerConfig, Permeability, ServerError};
use tokio::sync::{mpsc::Receiver, Notify, RwLock};
use tokio::task::JoinHandle;

use crate::packets::prelude::*;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use tracing::{error, instrument, trace, warn};

pub struct Dispatcher {
    topics: Arc<RwLock<Topics>>,
    cfg: Arc<MqttServerConfig>,
    shutdown: Arc<Notify>,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    incoming: Receiver<(String, Packet)>,
}

impl Dispatcher {
    pub fn new(
        topics: Arc<RwLock<Topics>>,
        cfg: Arc<MqttServerConfig>,
        shutdown: Arc<Notify>,
        clients: Arc<RwLock<HashMap<String, Client>>>,
        incoming: Receiver<(String, Packet)>,
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
        let mut response = Publish::new(topic).unwrap();
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
        response.set_payload(publish.payload());
        let resp = response.build();
        if let Some(ids) = self.topics.read().await.get_subscribed(topic) {
            trace!("Clients registers at {} are {:?}", topic, ids);
            let clients = self.clients.read().await;
            let strict_encryption = clients.get(client).unwrap().encrypted()
                && self.cfg.channel_permeability == Permeability::Strict;
            for id in ids {
                if let Some(c) = clients.get(id) {
                    if strict_encryption && !c.encrypted() {
                        continue;
                    }
                    if c.send(resp.clone()).is_err() {
                        error!(clientid = id.as_str(), "Internal Error: tx closed");
                    };
                }
            }
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn process_subscribe(&mut self, client: &str, sub: Subscribe) -> Result<(), ServerError> {
        trace!("Processing a subscribe packet");
        let ident = sub.packet_identifier();
        for (k, _) in sub.props_iter() {
            match k {
                Property::SubscriptionIdentifier => return self.unimplemented(client).await,
                Property::UserProperty => {
                    warn!(clientid = client, "Received unknown user property")
                }
                _ => error!(
                    "Internal Error: {:?} should not be part of publish packet",
                    k
                ),
            }
        }
        let mut suback = SubAck::new(ident);
        // it is important to drop topics as soon as possible because this is
        // write RAII
        let mut topics = self.topics.write().await;
        for (topic, options) in sub.topics_iter() {
            let qos: QoS = (*options).try_into()?;
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
                suback.add_reason_code(SubAckReasonCode::ImplementationSpecificError);
            }
            topics.subscribe(topic, client);
            match qos {
                QoS::QoS0 => suback.add_reason_code(SubAckReasonCode::GrantedQoS0),
                QoS::QoS1 => suback.add_reason_code(SubAckReasonCode::GrantedQoS1),
                QoS::QoS2 => suback.add_reason_code(SubAckReasonCode::GrantedQoS2),
            }
        }
        let clients = self.clients.read().await;
        if let Some(c) = clients.get(client) {
            if c.send(suback.build()).is_err() {
                error!(clientid = client, "Internal Error: tx closed");
            }
        }
        //this is important because we want more threads to have access to topics once we don't
        //have to write to it
        drop(topics);

        Ok(())
    }
    async fn process_packet(&mut self, client: &str, packet: Packet) -> Result<(), ServerError> {
        match packet {
            Packet::Publish(publish) => self.process_publish(client, publish).await,
            Packet::Subscribe(sub) => self.process_subscribe(client, sub).await,
            _ => self.unimplemented(client).await,
        }
    }
    async fn process_forever(mut self) {
        loop {
            let (client, packet) = match self.incoming.recv().await {
                Some(data) => data,
                None => {
                    warn!("incomming tx is closed");
                    break;
                }
            };
            if let Err(e) = self.process_packet(&client, packet).await {
                error!(clientid = &*client, "{:?}", e);
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
