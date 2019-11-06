#![deny(warnings)]

extern crate bincode;
#[macro_use]
extern crate clap;
extern crate crossbeam_channel;
extern crate futures;
extern crate hex_literal;
extern crate hyper;
extern crate kafka;
#[macro_use]
extern crate log;
extern crate rustls;
extern crate serde;
extern crate sled;
extern crate tokio;
extern crate tokio_rustls;
extern crate tokio_tcp;

use std::{io, sync, time::Duration};
use std::thread;

use clap::App;
use crossbeam_channel::{Receiver, Sender, unbounded};
use hyper::{Client, Server};
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use kafka::producer::{Producer, RequiredAcks};
use sled::Config;
use tokio_rustls::ServerConfigExt;

use handlers::{handler, kafka_event_handler, kafka_incoming_event_handler, Message};
use utils::pki_utils::{load_certs, load_private_key};

mod handlers;
mod utils;

fn main() {
    env_logger::init();
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    info!("Application arguments {:?}", matches);

    let external_address = matches.value_of("address").unwrap();
    let tls_port: u16 = matches.value_of("tlsport").unwrap_or_default().parse().expect("Unable to parse socket port");
    let plain_port: u16 = matches.value_of("plainport").unwrap_or_default().parse().expect("Unable to parse socket port");
    let kafka_sending_topic = matches.value_of("kafka_sending_topic").unwrap().to_owned();
    let kafka_broker_address = matches.value_of("kafka_broker").unwrap();
    let kafka_receiving_topic = matches.value_of("kafka_receiving_topic").unwrap().to_owned();
    let kafka_receiving_group = matches.value_of("kafka_receiving_group").unwrap_or_default();
    let http_tls_certificate = matches.value_of("http_tls_certificate").unwrap_or_default();
    let http_tls_key = matches.value_of("http_tls_key").unwrap();

    let tls_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), tls_port);
    let plain_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), plain_port);

    info!("Using kafka broker on {}", kafka_broker_address);
    let kafka_broker_address = vec!(String::from(kafka_broker_address));
    info!("Tls port Listening on {}", tls_socket_addr);
    info!("Plain port Listening on {}", plain_socket_addr);

    let (tls_tx, tls_rx): (Sender<Message>, Receiver<Message>) = unbounded();
    let (plain_tx, plain_rx): (Sender<Message>, Receiver<Message>) = unbounded();

    let config = Config::new().temporary(true);
    let db = config.open().unwrap();
    let client = Client::new();

    let tls_cfg = {
        // Load public certificate.
        let certs = load_certs(&http_tls_certificate).unwrap();
        // Load private key.
        let key = load_private_key(&http_tls_key).unwrap();
        // Do not use client certificate authentication.
        let mut cfg = rustls::ServerConfig::new(rustls::NoClientAuth::new());
        // Select a certificate to use.
        cfg.set_single_cert(certs, key)
            .map_err(|e| warn!("Problem with a cert {:?}", e)).unwrap();
        sync::Arc::new(cfg)
    };

    // Create a TCP listener via tokio.
    let tcp = tokio_tcp::TcpListener::bind(&tls_socket_addr).unwrap();

    // Prepare a long-running future stream to accept and serve cients.
    let tls = tcp.incoming().and_then(move |s| tls_cfg.accept_async(s))
        .then(|r| match r {
            Ok(x) => Ok::<_, io::Error>(Some(x)),
            Err(_e) => {
                println!("[!] Voluntary server halt due to client-connection error...");
                // Errors could be handled here, instead of server aborting.
                // Ok(None)
                Err(_e)
            }
        }).filter_map(|x| x);


    let mut plain_producer =
        Producer::from_hosts(kafka_broker_address.to_owned())
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();

    let mut tls_producer =
        Producer::from_hosts(kafka_broker_address.to_owned())
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();

    let mut consumer =
        Consumer::from_hosts(kafka_broker_address.to_owned())
            .with_topic(kafka_receiving_topic.to_owned())
            .with_fallback_offset(FetchOffset::Earliest)
            .with_group(kafka_receiving_group.to_owned())
            .with_offset_storage(GroupOffsetStorage::Kafka)
            .create()
            .unwrap();

    let server = Server::bind(&plain_socket_addr)
        .serve(move || {
            let inner_txx = plain_tx.clone();
            service_fn(move |req| handler(req, inner_txx.clone()))
        }
        ).map_err(|e| warn!("server error: {}", e));


    let tls_server = Server::builder(tls)
        .serve(move || {
            let inner_txx = tls_tx.clone();
            service_fn(move |req| handler(req, inner_txx.clone()))
        }
        ).map_err(|e| warn!("server error: {}", e));


    let db_clone = db.clone();
    let tls_kafka_sending_topic = kafka_sending_topic.clone();
    let tls_kafka_receiveing_topic = kafka_receiving_topic.clone();
    let kafka_sending_thread = thread::spawn(move || {
        let sending_topic = kafka_sending_topic.clone();
        let receiving_topic = kafka_receiving_topic.clone();
        loop {
            kafka_event_handler(&plain_rx, &mut plain_producer, &sending_topic, &receiving_topic, &db_clone);

        }
    });
    let db_clone = db.clone();
    let kafka_tls_sending_thread = thread::spawn(move || {
        let sending_topic = tls_kafka_sending_topic.clone();
        let receiving_topic = tls_kafka_receiveing_topic.clone();
        loop {
            kafka_event_handler(&tls_rx, &mut tls_producer, &sending_topic, &receiving_topic, &db_clone);
        }
    });

    let kafka_receiving_thread = thread::spawn(move || {
        loop {
            kafka_incoming_event_handler(consumer.poll().unwrap().iter(), &client, &mut consumer, &db);
        }
    });

    let tls_server = thread::spawn(move || { hyper::rt::run(tls_server) });
    let plain_server = thread::spawn(move || { hyper::rt::run(server); });

    kafka_sending_thread.join().unwrap();
    kafka_tls_sending_thread.join().unwrap();
    kafka_receiving_thread.join().unwrap();
    plain_server.join().unwrap_err();
    tls_server.join().unwrap_err();
}

