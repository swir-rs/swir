use std::borrow::Borrow;
use std::thread;
use std::thread::JoinHandle;

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::FutureExt;
use futures::stream::StreamExt;
use natsclient::{AuthenticationStyle, Client, ClientOptions};
use rdkafka::message::ToBytes;
use sled::{Db, IVec};

use crate::utils;

use super::super::utils::structs;
use super::super::utils::structs::*;

pub type Result<T> = std::result::Result<T, String>;


pub async fn configure_broker(broker_address: String, sending_topic: String, receiving_topic: String, _receiving_group: String, db: Db, rx: mpsc::Receiver<RestToMessagingContext>, tx: mpsc::Sender<utils::structs::MessagingToRestContext>) {
    let cluster = vec!(broker_address);

    let opts = ClientOptions::builder()
        .cluster_uris(cluster)
        .authentication(AuthenticationStyle::Anonymous)
        .build();
    match opts {
        Ok(opts) => {
            let nats_client = Client::from_options(opts).map_err(|e: natsclient::error::Error| String::from(format!("Nats Error {:?}", e)));
            match nats_client {
                Ok(client) => {
                    let connection_result = client.connect().map_err(|e: natsclient::error::Error| String::from(format!("Nats Error {:?}", e)));
                    match connection_result {
                        Ok(result) => {
                            info!("Connected to nats");
                            let nats_client_local = client.clone();
                            let db_local = db.clone();
                            let f1 = async { nats_incoming_event_handler(client, tx, db).await };
                            let f2 = async { nats_event_handler(rx, nats_client_local, &sending_topic, &receiving_topic, db_local).await };
                            let (_r1, _r2) = futures::join!(f1,f2);
                        },
                        Err(e) => warn!("Can't connect to NATS")
                    }
                }
                Err(e) => warn!("Can't connect to NATS")
            }
        }
        Err(e) => warn!("Can't connect to NATS")
    }
}

fn create_receiving_thread_nats(nats_client: natsclient::Client, tx: mpsc::Sender<utils::structs::MessagingToRestContext>, db: Db) {
    nats_incoming_event_handler(nats_client, tx, db);
}

pub async fn nats_event_handler(mut rx: mpsc::Receiver<RestToMessagingContext>, nats: Client, publish_topic: &str, subscribe_topic: &str, db: Db) {
    while let Some(job) = rx.next().await {
        let sender = job.sender;
        match job.job {
            Job::Subscribe(value) => {
                let req = value;
                info!("New registration  {:?}", req);
                if let Err(e) = db.insert(subscribe_topic, IVec::from(req.endpoint.url.as_bytes())) {
                    warn!("Can't store registration {:?}", e);
                    if let Err(e) = sender.send(structs::MessagingResult { status: 1, result: e.to_string() }) {
                        warn!("Can't send response back {:?}", e);
                    }
                } else {
                    if let Err(e) = sender.send(structs::MessagingResult { status: 1, result: "All is good".to_string() }) {
                        warn!("Can't send response back {:?}", e);
                    }
                };
            }

            Job::Publish(value) => {
                let req = value;
                let foo = nats.publish(publish_topic, &req.payload.to_bytes(), None);
                match foo {
                    Ok(_) => {
                        sender.send(structs::MessagingResult { status: 1, result: "NATS is good".to_string() });
                    }
                    Err(e) => {
                        sender.send(structs::MessagingResult { status: 1, result: e.to_string() });
                    }
                }
            }
        }
    }
}

pub async fn nats_incoming_event_handler(nats_client: natsclient::Client, tx: mpsc::Sender<utils::structs::MessagingToRestContext>, db: Db) {
    let mut tx_l = tx.clone();
    let s = nats_client.subscribe("Response", move |message| {
        let payload = &message.payload;
        let mut uri: String = String::from("");
        if let Ok(maybe_url) = db.get(&message.subject) {
            if let Some(url) = maybe_url {
                let vec = url.to_vec();
                uri = String::from_utf8_lossy(vec.borrow()).to_string();
            }
        }
        let (s, r) = oneshot::channel();
        let p = MessagingToRestContext {
            sender: s,
            payload: payload.to_vec(),
            uri: uri,
        };
        if let Err(e) = tx_l.try_send(p) {
            warn!("Error from the client {}", e)
        }
//        match r.recv() {
//            Ok(r) => info!("Response from the client {:?}", r),
//            Err(e) => warn!("Internal communication error  {}", e)
//        }
        Ok(())
    });
    if let Err(e) = s {
        warn!("Can't subscribe  {}", e)
    }
}


