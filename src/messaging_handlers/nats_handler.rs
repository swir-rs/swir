use std::borrow::Borrow;
use std::thread;
use std::thread::JoinHandle;

use crossbeam_channel::{bounded, Receiver, Sender};
use sled::{Db, IVec};

use natsclient::{AuthenticationStyle, Client, ClientOptions};

use crate::utils;

use super::super::utils::structs;
use super::super::utils::structs::*;

pub type Result<T> = std::result::Result<T, String>;


pub fn configure_broker(broker_address: String, sending_topic: String, receiving_topic: String, _receiving_group: String, db: Db, rx: Receiver<RestToMessagingContext>, tx: Sender<utils::structs::MessagingToRestContext>) -> Result<Vec<JoinHandle<()>>> {
    let cluster = vec!(broker_address);

    let opts = ClientOptions::builder()
        .cluster_uris(cluster)
        .authentication(AuthenticationStyle::Anonymous)
        .build()?;

    let nats_client = Client::from_options(opts).map_err(|e: natsclient::error::Error| String::from(format!("Nats Error {:?}", e)))?;
    nats_client.connect().map_err(|e: natsclient::error::Error| String::from(format!("Nats Error {:?}", e)))?;


    let mut threads: Vec<JoinHandle<()>> = vec!();

    let nats_client_local = nats_client.clone();
    let db_local = db.clone();
    let receive_thread = thread::spawn(move || {
        create_receiving_thread_nats(nats_client_local, tx, db_local)
    });

    let nats_client_local = nats_client.clone();
    let db_local = db.clone();
    let send_thread = thread::spawn(move || {
        nats_event_handler(&rx, nats_client_local, &sending_topic, &receiving_topic, db_local);
    });

    threads.push(send_thread);
    threads.push(receive_thread);
    Ok(threads)
}

fn create_receiving_thread_nats(nats_client: natsclient::Client, tx: Sender<utils::structs::MessagingToRestContext>, db: Db) {
    nats_incoming_event_handler(nats_client, tx, db);
}

pub fn nats_event_handler(rx: &Receiver<RestToMessagingContext>, nats: Client, publish_topic: &str, subscribe_topic: &str, db: Db) {
    let jobs = rx.iter();
    for j in jobs {
        let sender = j.sender;
        match j.job {
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
                let foo = nats.publish(publish_topic, &req.payload.as_bytes(), None).map(|_| {
                    sender.send(structs::MessagingResult { status: 1, result: "NATS is good".to_string() })
                }).map_err(|e| {
                    warn!("hmmm something is very wrong here. it seems that the channel has been closed");
                    sender.send(structs::MessagingResult { status: 1, result: e.to_string() })
                });
                if let Err(_) = foo {
                    warn!("hmmm something is very wrong here. it seems that the channel has been closed");
                }
            }
        }
    }
}

pub fn nats_incoming_event_handler(nats_client: natsclient::Client, tx: Sender<utils::structs::MessagingToRestContext>, db: Db) {
    let s = nats_client.subscribe("Response", move |message| {
        let payload = &message.payload;
        let mut uri: String = String::from("");
        if let Ok(maybe_url) = db.get(&message.subject) {
            if let Some(url) = maybe_url {
                let vec = url.to_vec();
                uri = String::from_utf8_lossy(vec.borrow()).to_string();
            }
        }
        let (s, r) = bounded(1);
        let p = MessagingToRestContext {
            sender: s,
            payload: payload.to_vec(),
            uri: uri,
        };
        if let Err(e) = tx.send(p) {
            warn!("Error from the client {}", e)
        }
        match r.recv() {
            Ok(r) => info!("Response from the client {:?}", r),
            Err(e) => warn!("Internal communication error  {}", e)
        }
        Ok(())
    });
    if let Err(e) = s {
        warn!("Can't subscribe  {}", e)
    }
}


