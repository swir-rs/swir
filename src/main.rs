//#![deny(warnings)]
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;


use std::{io, sync};
use std::thread;

use clap::App;
use crossbeam_channel::{Receiver, Sender, unbounded};
use futures::future;
use hyper::rt::{Future, Stream};
use hyper::Server;
use hyper::service::service_fn;
use sled::Config;
use tokio_rustls::ServerConfigExt;

use http_handler::client_handler;
use http_handler::handler;
use utils::pki_utils::{load_certs, load_private_key};

mod messaging_handlers;
mod http_handler;
mod utils;


fn main() {
    env_logger::init();
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    info!("Application arguments {:?}", matches);



    let external_address = matches.value_of("address").unwrap();
    let tls_port: u16 = matches.value_of("tlsport").unwrap_or_default().parse().expect("Unable to parse socket port");
    let plain_port: u16 = matches.value_of("plainport").unwrap_or_default().parse().expect("Unable to parse socket port");
    let sending_topic = matches.value_of("sending_topic").unwrap();
    let broker_address = matches.value_of("broker").unwrap();
    let command = matches.value_of("execute_command").unwrap();
    let receiving_topic = matches.value_of("receiving_topic").unwrap();


    let receiving_group = matches.value_of("receiving_group").unwrap_or_default();
    let http_tls_certificate = matches.value_of("http_tls_certificate").unwrap_or_default();
    let http_tls_key = matches.value_of("http_tls_key").unwrap();

    let tls_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), tls_port);
    let plain_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), plain_port);

    info!("Using kafka broker on {}", broker_address);
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

    let (rest_to_msg_tx, rest_to_msg_rx): (Sender<utils::structs::RestToMessagingContext>, Receiver<utils::structs::RestToMessagingContext>) = unbounded();
    let (msg_to_rest_tx, msg_to_rest_rx): (Sender<utils::structs::MessagingToRestContext>, Receiver<utils::structs::MessagingToRestContext>) = unbounded();

    let threads = messaging_handlers::configure_broker(broker_address.to_string(), sending_topic.to_string(), receiving_topic.to_string(), receiving_group.to_string(), db.clone(), rest_to_msg_rx.clone(), msg_to_rest_tx.clone()).unwrap();

    let tx = rest_to_msg_tx.clone();

    let server = Server::bind(&plain_socket_addr)
        .serve(move || {
            let tx = tx.clone();
            service_fn(move |req| handler(req, tx.clone()))
        }
        ).map_err(|e| warn!("server error: {}", e));


    let tx = rest_to_msg_tx.clone();
    let tls_server = Server::builder(tls)
        .serve(move || {
            let tx = tx.clone();
            service_fn(move |req| handler(req, tx.clone()))
        }
        ).map_err(|e| warn!("server error: {}", e));


    let https_server_runtime = thread::spawn(move || { hyper::rt::run(tls_server) });
    let http_plain_server_runtime = thread::spawn(move || { hyper::rt::run(server); });
    let http_client_runtime = thread::spawn(move || { hyper::rt::run(future::lazy(move || { client_handler(msg_to_rest_rx) })) });

    utils::command_utils::run_java_command(command.to_string());

    for t in threads {
        t.join().unwrap()
    }

    https_server_runtime.join().unwrap_err();
    http_plain_server_runtime.join().unwrap_err();
    http_client_runtime.join().unwrap_err();
}

