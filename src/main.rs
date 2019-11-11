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
extern crate rustls;
extern crate serde;
extern crate sled;
extern crate tokio;
extern crate tokio_rustls;
extern crate tokio_tcp;

use std::{io, sync};
use std::thread;

use clap::App;
use crossbeam_channel::{Receiver, Sender, unbounded};
use hyper::Server;
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use sled::Config;
use tokio_rustls::ServerConfigExt;

use handlers::handler;
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
    let kafka_sending_topic = matches.value_of("kafka_sending_topic").unwrap();
    let kafka_broker_address = matches.value_of("kafka_broker").unwrap();
    let kafka_receiving_topic = matches.value_of("kafka_receiving_topic").unwrap();


    let kafka_receiving_group = matches.value_of("kafka_receiving_group").unwrap_or_default();
    let http_tls_certificate = matches.value_of("http_tls_certificate").unwrap_or_default();
    let http_tls_key = matches.value_of("http_tls_key").unwrap();

    let tls_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), tls_port);
    let plain_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), plain_port);

    info!("Using kafka broker on {}", kafka_broker_address);
    let kafka_broker_address = kafka_broker_address;
    info!("Tls port Listening on {}", tls_socket_addr);
    info!("Plain port Listening on {}", plain_socket_addr);



    let config = Config::new().temporary(true);
    let db = config.open().unwrap();


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

    let (plain_tx, plain_rx): (Sender<handlers::InternalMessage>, Receiver<handlers::InternalMessage>) = unbounded();

    let db_clone = db.clone();

    let threads = handlers::configure_broker(kafka_broker_address.to_string(), kafka_sending_topic.to_string(), kafka_receiving_topic.to_string(), kafka_receiving_group.to_string(), db_clone, plain_rx.clone());

    let plain_tx1 = plain_tx.clone();

    let server = Server::bind(&plain_socket_addr)
        .serve(move || {
            let inner_txx = plain_tx1.clone();
            service_fn(move |req| handler(req, inner_txx.clone()))
        }
        ).map_err(|e| warn!("server error: {}", e));


    let plain_tx2 = plain_tx.clone();
    let tls_server = Server::builder(tls)
        .serve(move || {
            let inner_txx = plain_tx2.clone();
            service_fn(move |req| handler(req, inner_txx.clone()))
        }
        ).map_err(|e| warn!("server error: {}", e));




    let tls_server = thread::spawn(move || { hyper::rt::run(tls_server) });
    let plain_server = thread::spawn(move || { hyper::rt::run(server); });

    for t in threads {
        t.join().unwrap()
    }
    plain_server.join().unwrap_err();
    tls_server.join().unwrap_err();
}

