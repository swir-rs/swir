//#![deny(warnings)]

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
extern crate rdkafka;
extern crate rdkafka_sys;
extern crate rustls;
extern crate serde;
extern crate sled;
extern crate tokio;
extern crate tokio_rustls;
extern crate tokio_tcp;

use std::{io, sync};
use std::borrow::Borrow;
use std::thread;

use clap::App;
use crossbeam_channel::{Receiver, Sender, unbounded};
use futures::future;
use hyper::{Client, Server};
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{Consumer, ConsumerContext, Rebalance};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::error::KafkaResult;
use rdkafka::producer::FutureProducer;
use sled::Config;
use tokio_rustls::ServerConfigExt;

use handlers::{handler, kafka_event_handler, kafka_incoming_event_handler};
use handlers::structs::CustomContext;
use utils::pki_utils::{load_certs, load_private_key};

mod handlers;

mod utils;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, rebalance: &Rebalance) {
        info!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, rebalance: &Rebalance) {
        info!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(
        &self,
        result: KafkaResult<()>,
        _offsets: *mut rdkafka_sys::RDKafkaTopicPartitionList,
    ) {
        info!("Committing offsets: {:?}", result);
    }
}


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
    let kafka_broker_address = kafka_broker_address;
    info!("Tls port Listening on {}", tls_socket_addr);
    info!("Plain port Listening on {}", plain_socket_addr);

    let (tls_tx, tls_rx): (Sender<handlers::InternalMessage>, Receiver<handlers::InternalMessage>) = unbounded();
    let (plain_tx, plain_rx): (Sender<handlers::InternalMessage>, Receiver<handlers::InternalMessage>) = unbounded();

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

    let context = CustomContext;
    let plain_producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", kafka_broker_address)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    let tls_producer =
        ClientConfig::new()
            .set("bootstrap.servers", kafka_broker_address)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer creation error");

    let consumer: StreamConsumer<CustomContext> = ClientConfig::new()
        .set("group.id", kafka_receiving_group)
        .set("bootstrap.servers", kafka_broker_address)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed");


    let mut topics: Vec<&str> = vec!();
    topics.push(kafka_receiving_topic.as_str());
    let tt = topics.borrow();
    consumer.subscribe(tt).unwrap();

//    let mut consumer =
//        Consumer::from_hosts(kafka_broker_address.to_owned())
//            .with_topic(kafka_receiving_topic.to_owned())
//            .with_fallback_offset(FetchOffset::Earliest)
//            .with_group(kafka_receiving_group.to_owned())
//            .with_offset_storage(GroupOffsetStorage::Kafka)
//            .create()
//            .unwrap();

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
            kafka_event_handler(&plain_rx, &plain_producer, &sending_topic, &receiving_topic, &db_clone);

        }
    });
    let db_clone = db.clone();
    let kafka_tls_sending_thread = thread::spawn(move || {
        let sending_topic = tls_kafka_sending_topic.clone();
        let receiving_topic = tls_kafka_receiveing_topic.clone();
        loop {
            kafka_event_handler(&tls_rx, &tls_producer, &sending_topic, &receiving_topic, &db_clone);
        }
    });

    let kafka_receiving_thread = thread::spawn(move || {
        let f = {
            kafka_incoming_event_handler(&consumer, &client, &db);
            future::ok(())
        };
        hyper::rt::run(f);
    });

    let tls_server = thread::spawn(move || { hyper::rt::run(tls_server) });
    let plain_server = thread::spawn(move || { hyper::rt::run(server); });

    kafka_sending_thread.join().unwrap();
    kafka_tls_sending_thread.join().unwrap();
    kafka_receiving_thread.join().unwrap();
    plain_server.join().unwrap_err();
    tls_server.join().unwrap_err();
}

