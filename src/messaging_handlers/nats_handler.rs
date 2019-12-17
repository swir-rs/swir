use std::sync::Arc;

use futures::lock::Mutex;
use futures::stream::StreamExt;
use nats::*;
use sled::{Db, IVec};
use tokio::sync::mpsc;
use tokio::task;

use async_trait::async_trait;

use crate::messaging_handlers::Broker;
use crate::utils;

use super::super::utils::structs;
use super::super::utils::structs::*;

pub struct NatsBroker {
    pub broker_address: Vec<String>,
    pub consumer_topics: Vec<String>,
    pub consumer_groups: Vec<String>,
    pub producer_topics: Vec<String>,
    pub db: Db,
    pub rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    pub tx: Box<mpsc::Sender<MessagingToRestContext>>,
}


#[async_trait]
impl Broker for NatsBroker {
    async fn configure_broker(&self) {
        let cluster = self.broker_address.clone();

        let mut incoming_client = nats::Client::new(cluster.clone()).unwrap();
        let outgoing_client = nats::Client::new(cluster).unwrap();

        let consumer_group = self.consumer_groups.get(0).unwrap();
        let consumer_topic = self.consumer_topics.get(0).unwrap();
        let producer_topic = self.producer_topics.get(0).unwrap();

        incoming_client.set_name(&consumer_group);
        incoming_client
            .subscribe(consumer_topic.as_str(), Some(&consumer_group))
            .unwrap();
        info!("NATS subscribed and connected");
        let db_local = self.db.clone();
        let f1 = async { nats_incoming_event_handler(incoming_client, self.tx.clone(), self.db.clone()).await };
        let f2 = async {
            nats_event_handler(
                self.rx.clone(),
                outgoing_client,
                producer_topic,
                consumer_topic,
                db_local,
            )
                .await
        };
        let (_r1, _r2) = futures::join!(f1, f2);
    }
}

async fn nats_event_handler(
    rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    mut nats: Client,
    producer_topic: &str,
    consumer_topic: &str,
    db: Db,
) {
    let mut rx = rx.lock().await;
    while let Some(job) = rx.next().await {
        let sender = job.sender;
        match job.job {
            Job::Subscribe(value) => {
                let req = value;
                info!("New registration  {:?}", req);
                if let Err(e) = db.insert(consumer_topic, IVec::from(req.endpoint.url.as_bytes()))
                {
                    warn!("Can't store registration {:?}", e);
                    if let Err(e) = sender.send(structs::MessagingResult {
                        status: 1,
                        result: e.to_string(),
                    }) {
                        warn!("Can't send response back {:?}", e);
                    }
                } else {
                    if let Err(e) = sender.send(structs::MessagingResult {
                        status: 1,
                        result: "All is good".to_string(),
                    }) {
                        warn!("Can't send response back {:?}", e);
                    }
                };
            }

            Job::Publish(value) => {
                let req = value;
                let foo = nats.publish(producer_topic, &req.payload);
                match foo {
                    Ok(_) => {
                        sender.send(structs::MessagingResult {
                            status: 1,
                            result: "NATS is good".to_string(),
                        });
                    }
                    Err(e) => {
                        sender.send(structs::MessagingResult {
                            status: 1,
                            result: e.to_string(),
                        });
                    }
                }
            }
        }
    }
}

async fn nats_incoming_event_handler(
    mut client: Client,
    tx: Box<mpsc::Sender<utils::structs::MessagingToRestContext>>,
    db: Db,
) {
    let join = task::spawn_blocking(move || {
        info!("Waiting for events ");
        for event in client.events() {
            let db = db.clone();
            let mut tx = tx.clone();
            tokio::spawn(async move {
                let mut uri: String = String::from("");
                if let Ok(maybe_url) = db.clone().get(&event.subject) {
                    if let Some(url) = maybe_url {
                        let vec = url.to_vec();
                        uri = String::from_utf8_lossy(&vec).to_string();
                    }
                }

                let (s, _r) = futures::channel::oneshot::channel();
                let p = MessagingToRestContext {
                    sender: s,
                    payload: event.msg,
                    uri: uri,
                };
                if let Err(e) = tx.try_send(p) {
                    warn!("Error from the client {}", e)
                }
            });
        }
    });
    let _res = join.await;
}
