use std::collections::HashMap;
use std::sync::Arc;

use futures::lock::Mutex;
use futures::stream::StreamExt;
use nats::*;
use rdkafka::message::ToBytes;
use sled::{Db, IVec};
use tokio::sync::mpsc;
use tokio::task;

use async_trait::async_trait;

use crate::messaging_handlers::Broker;
use crate::utils::config::Nats;

use super::super::utils::structs;
use super::super::utils::structs::*;

#[derive(Debug)]
pub struct NatsBroker {
    pub nats: Nats,
    pub db: Db,
    pub rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    pub tx: Box<HashMap<CustomerInterfaceType, Box<mpsc::Sender<MessagingToRestContext>>>>,
}

impl NatsBroker {
    async fn nats_event_handler(&self, mut nats: Client) {
        let mut rx = self.rx.lock().await;
        while let Some(job) = rx.next().await {
            let sender = job.sender;
            match job.job {
                Job::Subscribe(value) => {
                    let req = value;
                    info!("New registration  {:?}", req);

                    let maybe_topic = self.nats.get_consumer_topic_for_client_topic(&req.client_topic);
                    if let Some(topic) = maybe_topic {
                        let s = serde_json::to_string(&req).unwrap();
                        if let Err(e) = self.db.insert(topic, IVec::from(s.as_bytes())) {
                            warn!("Can't store registration {:?}", e);
                            if let Err(e) = sender.send(structs::MessagingResult {
                                status: BackendStatusCodes::Error(e.to_string()),
                            }) {
                                warn!("Can't send response back {:?}", e);
                            }
                        } else {
                            if let Err(e) = sender.send(structs::MessagingResult {
                                status: BackendStatusCodes::Ok("NATS is good".to_string()),
                            }) {
                                warn!("Can't send response back {:?}", e);
                            }
                        }
                    } else {
                        warn!("Can't find topic {:?}", req);
                        if let Err(e) = sender.send(structs::MessagingResult {
                            status: BackendStatusCodes::NoTopic("Can't find subscribe topic".to_string()),
                        }) {
                            warn!("Can't send response back {:?}", e);
                        }
                    }
                }

                Job::Publish(value) => {
                    let req = value;
                    let maybe_topic = self.nats.get_producer_topic_for_client_topic(&req.client_topic);
                    if let Some(topic) = maybe_topic {
                        let foo = nats.publish(&topic, &req.payload);
                        match foo {
                            Ok(_) => {
                                sender.send(structs::MessagingResult {
                                    status: BackendStatusCodes::Ok("NATS is good".to_string()),
                                });
                            }
                            Err(e) => {
                                sender.send(structs::MessagingResult {
                                    status: BackendStatusCodes::Error(e.to_string()),
                                });
                            }
                        }
                    } else {
                        warn!("Can't find topic {:?}", req);
                        if let Err(e) = sender.send(structs::MessagingResult {
                            status: BackendStatusCodes::NoTopic("Can't find subscribe topic".to_string()),
                        }) {
                            warn!("Can't send response back {:?}", e);
                        }
                    }
                }
            }
        }
    }

    async fn nats_incoming_event_handler(&self, mut client: Client) {
        let db = self.db.clone();
        let tx = self.tx.clone();

        let join = task::spawn_blocking(move || {
            info!("Waiting for events ");
            for event in client.events() {
                if let Ok(maybe_url) = db.get(&event.subject) {
                    if let Some(url) = maybe_url {
                        let vec = url.to_bytes();
                        let subscribe_request: SubscribeRequest = serde_json::from_slice(&vec).unwrap();

                        let mut tx = tx.get(&subscribe_request.client_interface_type).unwrap().clone();
                        let (s, _r) = futures::channel::oneshot::channel();
                        let p = MessagingToRestContext {
                            sender: s,
                            payload: event.msg,
                            uri: subscribe_request.endpoint.url.clone(),
                        };
                        if let Err(e) = tx.try_send(p) {
                            warn!("Error from the client {}", e)
                        }
                    }
                }
            }
        });
        let _res = join.await;
    }
}

#[async_trait]
impl Broker for NatsBroker {
    async fn configure_broker(&self) {
        info!("Configuring NATS broker {:?} ", self);
        let cluster = self.nats.brokers.clone();
        let mut incoming_client = nats::Client::new(cluster.clone()).unwrap();
        let outgoing_client = nats::Client::new(cluster).unwrap();

        let mut consumer_topics = vec![];
        let mut consumer_groups = vec![];
        for ct in self.nats.consumer_topics.iter() {
            consumer_topics.push(ct.consumer_topic.clone());
            consumer_groups.push(ct.consumer_group.clone());
        }

        let mut producer_topics = vec![];
        for pt in self.nats.producer_topics.iter() {
            producer_topics.push(pt.producer_topic.clone());
        }

        let consumer_group = consumer_groups.get(0).unwrap();
        let consumer_topic = consumer_topics.get(0).unwrap();

        incoming_client.set_name(&consumer_group);
        incoming_client.subscribe(consumer_topic.as_str(), Some(&consumer_group)).unwrap();
        info!("NATS subscribed and connected");

        let f1 = async { self.nats_incoming_event_handler(incoming_client).await };
        let f2 = async { self.nats_event_handler(outgoing_client).await };
        let (_r1, _r2) = futures::join!(f1, f2);
    }
}
