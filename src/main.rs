//#![deny(warnings)]

#[macro_use]
extern crate clap;
extern crate futures;
extern crate hyper;
extern crate kafka;
#[macro_use]
extern crate log;
extern crate rustls;
extern crate tokio;
extern crate tokio_rustls;
extern crate tokio_tcp;


use std::{fs, io, str, sync, sync::Arc, time::Duration};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::thread;

use clap::App;
use futures::future;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use kafka::producer::{Producer, Record, RequiredAcks};
use rustls::internal::pemfile;
use tokio_rustls::ServerConfigExt;

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;


fn echo(req: Request<Body>, counter: &AtomicUsize , s:&Sender<usize>) -> BoxFut {
    let mut response = Response::new(Body::empty());
    counter.fetch_add(1, Ordering::Relaxed);
    info!("Counter {}", counter.load(Ordering::Relaxed));
    let headers= req.headers();
    info!("Headers {:?}", headers);
    let (parts,body) = req.into_parts();
    info!("Parts {:?}", parts);
    info!("Body {:?}", body);

    match (parts.method, parts.uri.path()) {
        // Serve some instructions at /
        (Method::GET, "/") => {
            *response.body_mut() = Body::from("Try POSTing data to /echo");
        }

        // Simply echo the body back to the client.
        (Method::POST, "/echo") => {
//            let res = body.concat2().wait().unwrap();
//            info!("{:?}", res);
//            producer.send(&Record::from_value("Request", body.as_bytes())).unwrap();
            info!("Sending {}", counter.load(Ordering::Relaxed));
            s.send(counter.load(Ordering::Relaxed)).unwrap();
            *response.body_mut() = body;
        }

        // Convert to uppercase before sending back to client.
        (Method::POST, "/echo/uppercase") => {
            let mapping = body.map(|chunk| {
                chunk
                    .iter()
                    .map(|byte| byte.to_ascii_uppercase())
                    .collect::<Vec<u8>>()
            });

            *response.body_mut() = Body::wrap_stream(mapping);
        }

        // Reverse the entire body before sending back to the client.
        //
        // Since we don't know the end yet, we can't simply stream
        // the chunks as they arrive. So, this returns a different
        // future, waiting on concatenating the full body, so that
        // it can be reversed. Only then can we return a `Response`.
        (Method::POST, "/echo/reversed") => {
            let reversed = body.concat2().map(move |chunk| {
                let body = chunk.iter().rev().cloned().collect::<Vec<u8>>();
                *response.body_mut() = Body::from(body);
                response
            });

            return Box::new(reversed);
        }

        // The 404 Not Found route...
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Box::new(future::ok(response))
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
    let kafka_receiving_topic = matches.value_of("kafka_receiving_topic").unwrap();
    let kafka_receiving_group = matches.value_of("kafka_receiving_group").unwrap();


    let tls_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), tls_port);
    let plain_socket_addr = std::net::SocketAddr::new(external_address.parse().unwrap(), plain_port);

    info!("Using kafka broker on {}", kafka_broker_address);
    let kafka_broker_address = vec!(String::from(kafka_broker_address));
    info!("Tls port Listening on {}", tls_socket_addr);
    info!("Plain port Listening on {}", plain_socket_addr);



    let (tx, rx):(Sender<usize>, Receiver<usize>) = mpsc::channel();
    let txx = tx.clone();


    let tls_cfg = {
        // Load public certificate.
        let certs = load_certs("cert_util/localhost.pem").unwrap();
        // Load private key.
        let key = load_private_key("cert_util/localhost.key").unwrap();
        // Do not use client certificate authentication.
        let mut cfg = rustls::ServerConfig::new(rustls::NoClientAuth::new());
        // Select a certificate to use.
        cfg.set_single_cert(certs, key)
            .map_err(|e| error(format!("{}", e))).unwrap();
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


    let mut producer =
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


    let request_counter = Arc::new(AtomicUsize::new(0));
    let request_counter_tls = Arc::clone(&request_counter);
    let request_counter_plain = Arc::clone(&request_counter);

    let tlsserver = Server::builder(tls)
        .serve(move || {
            let inner_rc = Arc::clone(&request_counter_tls);
            let inner_txx = mpsc::Sender::clone(&tx);
            service_fn(move |req| echo(req, &inner_rc,&mpsc::Sender::clone(&inner_txx)))
        }
        ).map_err(|e| warn!("server error: {}", e));

    let server = Server::bind(&plain_socket_addr)
        .serve(move || {
            let inner_rc = Arc::clone(&request_counter_plain);
            let inner_txx = mpsc::Sender::clone(&txx);
            service_fn(move |req| echo(req, &inner_rc, &mpsc::Sender::clone(&inner_txx)))
        }
        ).map_err(|e| warn!("server error: {}", e));


    let kafka_sending_thread = thread::spawn(move || {
        let topic = kafka_sending_topic.clone();
        loop {
            let i = rx.recv().unwrap();
            info!("Sending {}", i);
            producer.send(&Record::from_value(&topic, i.to_string())).unwrap();
        }
    });

    let kafka_receiving_thread = thread::spawn(move || {
        loop {
            for ms in consumer.poll().unwrap().iter() {
                for m in ms.messages() {
                    info!("Received message {:?}", std::str::from_utf8(m.value).unwrap());
                }
                let r = consumer.consume_messageset(ms);
                info!("Consumed result {:?}", r);
            }
            consumer.commit_consumed().unwrap();
        }
    });


    thread::spawn(move || { hyper::rt::spawn(tlsserver) });
    thread::spawn(move || { hyper::rt::spawn(server) });

    kafka_sending_thread.join().unwrap();
    kafka_receiving_thread.join().unwrap();

}


// Load public certificate from file.
fn load_certs(filename: &str) -> io::Result<Vec<rustls::Certificate>> {
    // Open certificate file.
    let certfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    pemfile::certs(&mut reader).map_err(|_| error("failed to load certificate".into()))
}

// Load private key from file.
fn load_private_key(filename: &str) -> io::Result<rustls::PrivateKey> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    let keys = pemfile::rsa_private_keys(&mut reader)
        .map_err(|_| error("failed to load private key".into()))?;
    if keys.len() != 1 {
        return Err(error("expected a single private key".into()));
    }
    Ok(keys[0].clone())
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}