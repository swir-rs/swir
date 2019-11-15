extern crate natsclient;

use std::borrow::Borrow;
use std::thread;
use std::thread::JoinHandle;

use crossbeam_channel::Receiver;
use http::HeaderValue;
use hyper::{Body, Method, Request, Uri};
use hyper::body::Payload;
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;
use hyper::rt::{Future, Stream};
use natsclient::{AuthenticationStyle, Client, ClientOptions};
use sled::{Db, IVec};

use super::super::utils::structs;
use super::super::utils::structs::*;

pub type Result<T> = std::result::Result<T, String>;


pub fn configure_broker(broker_address: String, sending_topic: String, receiving_topic: String, _receiving_group: String, db: Db, rx: Receiver<InternalMessage>) -> Result<Vec<JoinHandle<()>>> {
    let cluster = vec!(broker_address);

    let opts = ClientOptions::builder()
        .cluster_uris(cluster)
        .authentication(AuthenticationStyle::Anonymous)
        .build()?;

    let nats_client = Client::from_options(opts).map_err(|e: natsclient::error::Error| String::from(format!("Nats Error {:?}", e)))?;
    nats_client.connect().map_err(|e: natsclient::error::Error| String::from(format!("Nats Error {:?}", e)))?;

    let http_client: hyper::Client<HttpConnector<GaiResolver>, Body> = hyper::Client::new();

    let mut threads: Vec<JoinHandle<()>> = vec!();

    let nats_client_local = nats_client.clone();
    let db_local = db.clone();
    let receive_thread = thread::spawn(move || {
        create_receiving_thread_nats(nats_client_local, http_client, db_local)
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

fn create_receiving_thread_nats(nats_client: natsclient::Client, client: hyper::Client<HttpConnector<GaiResolver>, Body>, db: Db) {
    nats_incoming_event_handler(nats_client, client, db);
}

pub fn nats_event_handler(rx: &Receiver<InternalMessage>, nats: Client, publish_topic: &str, subscribe_topic: &str, db: Db) {
    let jobs = rx.iter();
    for j in jobs {
        let sender = j.sender;
        match j.job {
            Job::Subscribe(value) => {
                let req = value;
                info!("New registration  {:?}", req);
                if let Err(e) = db.insert(subscribe_topic, IVec::from(req.endpoint.url.as_bytes())) {
                    warn!("Can't store registration {:?}", e);
                    if let Err(e) = sender.send(structs::KafkaResult { result: e.to_string() }) {
                        warn!("Can't send response back {:?}", e);
                    }
                } else {
                    if let Err(e) = sender.send(structs::KafkaResult { result: "All is good".to_string() }) {
                        warn!("Can't send response back {:?}", e);
                    }
                };
            }

            Job::Publish(value) => {
                let req = value;
                info!("Kafka plain sending {:?}", req);

                let foo = nats.publish(publish_topic, &req.payload.as_bytes(), None).map(|_| {
                    sender.send(structs::KafkaResult { result: "NATS is good".to_string() })
                }).map_err(|e| {
                    warn!("hmmm something is very wrong here. it seems that the channel has been closed");
                    sender.send(structs::KafkaResult { result: e.to_string() })
                });
                if let Err(_) = foo {
                    warn!("hmmm something is very wrong here. it seems that the channel has been closed");
                }

            }
        }
    }
}

pub fn nats_incoming_event_handler(nats_client: natsclient::Client, client: hyper::Client<HttpConnector, Body>, db: Db) {
    let s = nats_client.subscribe("Response", move |message| {
        info!("NATS processing message");
        let payload = &message.payload;
        let mut uri: String = String::from("");
        if let Ok(maybe_url) = db.get(&message.subject) {
            if let Some(url) = maybe_url {
                let vec = url.to_vec();
                uri = String::from_utf8_lossy(vec.borrow()).to_string();
            }
        }

        let (s, r) = futures::sync::oneshot::channel::<u16>();
        let mut postreq = Request::new(Body::from(payload.to_owned()));
        *postreq.method_mut() = Method::POST;
        *postreq.uri_mut() = Uri::from(uri.parse().unwrap_or_default());
        postreq.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );


        info!("NATS trying to post");
        let cl = postreq.body_mut().content_length().unwrap().to_string();
        postreq.headers_mut().insert(
            hyper::header::CONTENT_LENGTH,
            HeaderValue::from_str(&cl).unwrap(),
        );

        let post = client.request(postreq).and_then(|res| {
            let status = res.status();
            info!("POST: {}", res.status());
            if let Err(e) = s.send(status.as_u16()) {
                warn!("Problem with an internal communication {:?}", e);
            }
            res.into_body().concat2()
        }).map(|_| {
            info!("Done.");
        }).map_err(|err| {
            warn!("Error {}", err);
        });
        hyper::rt::run(post);
        let result = r.map(|f| {
            if f != 200 {
                warn!("Error from the client {}", f)
            }
        }).wait();
        if let Err(e) = result {
            warn!("Error from the client {}", e)
        }
        Ok(())
    });
    if let Err(e) = s {
        warn!("Can't subscribe  {}", e)
    }
}


