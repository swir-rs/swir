use futures::stream::StreamExt;
use nats::*;
use sled::{Db, IVec};
use tokio::sync::mpsc;
use tokio::task;

use crate::utils;

use super::super::utils::structs;
use super::super::utils::structs::*;

pub async fn configure_broker(
    broker_address: Vec<String>,
    producer_topics: Vec<String>,
    consumer_topics: Vec<String>,
    consumer_groups: Vec<String>,
    db: Db,
    rx: mpsc::Receiver<RestToMessagingContext>,
    tx: mpsc::Sender<utils::structs::MessagingToRestContext>,
) {
    let cluster = broker_address;

    let mut incoming_client = nats::Client::new(cluster.clone()).unwrap();
    let outgoing_client = nats::Client::new(cluster).unwrap();

    let consumer_group = consumer_groups.get(0).unwrap();
    let consumer_topic = consumer_topics.get(0).unwrap();
    let producer_topic = producer_topics.get(0).unwrap();

    incoming_client.set_name(&consumer_group);
    incoming_client
        .subscribe(consumer_topic.as_str(), Some(&consumer_group))
        .unwrap();
    info!("NATS subscribed and connected");
    let db_local = db.clone();
    let f1 = async { nats_incoming_event_handler(incoming_client, tx, db).await };
    let f2 = async {
        nats_event_handler(
            rx,
            outgoing_client,
            producer_topic,
            consumer_topic,
            db_local,
        )
            .await
    };
    let (_r1, _r2) = futures::join!(f1, f2);
}

pub async fn nats_event_handler(
    mut rx: mpsc::Receiver<RestToMessagingContext>,
    mut nats: Client,
    producer_topic: &str,
    consumer_topic: &str,
    db: Db,
) {
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

pub async fn nats_incoming_event_handler(
    mut client: Client,
    tx: mpsc::Sender<utils::structs::MessagingToRestContext>,
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
