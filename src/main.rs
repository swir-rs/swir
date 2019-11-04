//#![deny(warnings)]

#[macro_use]
extern crate clap;
extern crate crossbeam_channel;
extern crate futures;
extern crate hyper;
extern crate kafka;
#[macro_use]
extern crate log;
extern crate rustls;
extern crate serde;
extern crate tokio;
extern crate tokio_rustls;
extern crate tokio_tcp;

use std::{fs, io, str, sync, time::Duration};
use std::thread;

use clap::App;
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel::unbounded;
use futures::future;
use http::HeaderValue;
use hyper::{Body, HeaderMap, Method, Request, Response, Server, StatusCode};
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use kafka::producer::{AsBytes, Producer, Record, RequiredAcks};
use rustls::internal::pemfile;
use serde::{Deserialize, Serialize};
use tokio_rustls::ServerConfigExt;

#[derive(Serialize, Deserialize, Debug)]
struct PublishRequest {
    key1: String,
    key2: String
}


#[derive(Serialize, Deserialize, Debug)]
struct RegisterRequest {
    key1: String,
    key2: String,
}



#[derive(Debug)]
struct KafkaResult {
    result: String
}

#[derive(Debug)]
enum Job {
    Register(RegisterRequest),
    Publish(PublishRequest),
}

#[derive(Debug)]
struct Message {
    job: Job,
    sender: Sender<KafkaResult>,
}


type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn validate_content_type(headers: &HeaderMap<HeaderValue>) -> Option<bool> {
    match headers.get(http::header::CONTENT_TYPE) {
        Some(header) => {
            if header == HeaderValue::from_static("application/json") {
                return Some(true)
            } else {
                return None
            }
        },
        None => return None
    }
}


fn handler(req: Request<Body>, sender: Sender<Message>) -> BoxFut {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;

    let headers= req.headers();
    info!("Headers {:?}", headers);

    if validate_content_type(headers).is_none() {
        return Box::new(future::ok(response))
    }

    let (parts,body) = req.into_parts();

    info!("Body {:?}", body);

    match (parts.method, parts.uri.path()) {
        (Method::POST, "/publish") => {
            let mapping = body.concat2()
                .map(|chunk| { chunk.to_vec() })
                .map(|payload| {
                    let p = &String::from_utf8_lossy(&payload.as_bytes());
                    info!("Payload is {:?}", &p);
                    serde_json::from_str(&p)
                }).map(move |p| {
                    match p {
                        Ok(json) => {
                            info!("{:?}", json);
                            let (local_tx, local_rx): (Sender<KafkaResult>, Receiver<KafkaResult>) = unbounded();
                            let job = Message { job: Job::Publish(json), sender: local_tx.clone() };
                            info!("About to send to kafka processor");
                            if let Err(e) = sender.send(job) {
                                warn!("Channel is dead {:?}", e);
                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                *response.body_mut() = Body::empty();
                                ;
                            }
                            info!("Waiting for response from kafka");
                            let r = local_rx.recv();
                            info!("Got result {:?}", r);
                            *response.body_mut() = Body::from("delivered");
                        },
                        Err(e) => {
                            warn!("{:?}", e);
                            *response.status_mut() = StatusCode::BAD_REQUEST;
                            *response.body_mut() = Body::from(e.to_string());
                        }
                    }
                    response
                });

            return Box::new(mapping);
        }

        (Method::POST, "/register") => {
            let mapping = body.concat2()
                .map(|chunk| { chunk.to_vec() })
                .map(|payload| {
                    let p = &String::from_utf8_lossy(&payload.as_bytes());
                    info!("Payload is {:?}", &p);
                    serde_json::from_str(&p)
                }).map(move |p| {
                    match p {
                        Ok(json) => {
                            info!("{:?}", json);
                            let (local_tx, local_rx): (Sender<KafkaResult>, Receiver<KafkaResult>) = unbounded();
                            let job = Message { job: Job::Register(json), sender: local_tx.clone() };
                            info!("About to send to kafka processor");
                            if let Err(e) = sender.send(job) {
                                warn!("Channel is dead {:?}", e);
                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                *response.body_mut() = Body::empty();
                                ;
                            }
                            info!("Waiting for response from kafka");
                            let r = local_rx.recv();
                            info!("Got result {:?}", r);
                            *response.body_mut() = Body::from("delivered");
                        },
                        Err(e) => {
                            warn!("{:?}", e);
                            *response.status_mut() = StatusCode::BAD_REQUEST;
                            *response.body_mut() = Body::from(e.to_string());
                        }
                    }
                    response
                });
            return Box::new(mapping);
        }
        // The 404 Not Found route...
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Box::new(future::ok(response))
}

fn kafka_event_handler(rx: &Receiver<Message>, kafka_producer: &mut Producer, topic: &String) {
    let job = rx.recv().unwrap();

    let sender = job.sender;
    match job.job {
        Job::Register(value) => {
            let req = value;
            info!("New registration  {:?}", req);
            if let Ok(event) = serde_json::to_string(&req) {
                if let Err(e) = sender.send(KafkaResult { result: "All is good".to_string() }) {
                    warn!("Can't send response back {:?}", e);
                }
            }
        },
        Job::Publish(value) => {
            let req = value;
            info!("Kafka plain sending {:?}", req);
            if let Ok(event) = serde_json::to_string(&req) {
                if let Err(e) = kafka_producer.send(&Record::from_value(&topic, event)) {
                    if let Err(e) = sender.send(KafkaResult { result: e.to_string() }) {
                        warn!("Can't send response back {:?}", e);
                    }
                } else {
                    if let Err(e) = sender.send(KafkaResult { result: "All is good".to_string() }) {
                        warn!("Can't send response back {:?}", e);
                    }
                }
            }
        }
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
    let kafka_receiving_topic = matches.value_of("kafka_receiving_topic").unwrap();
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



    let tls_cfg = {
        // Load public certificate.
        let certs = load_certs(&http_tls_certificate).unwrap();
        // Load private key.
        let key = load_private_key(&http_tls_key).unwrap();
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


    let tls_kafka_sending_topic = kafka_sending_topic.clone();
    let kafka_sending_thread = thread::spawn(move || {
        let topic = kafka_sending_topic.clone();
        loop {
            kafka_event_handler(&plain_rx, &mut plain_producer, &topic);

        }
    });

    let kafka_tls_sending_thread = thread::spawn(move || {
        let topic = tls_kafka_sending_topic.clone();
        loop {
            kafka_event_handler(&tls_rx, &mut tls_producer, &topic);
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
    let tls_server = thread::spawn(move || { hyper::rt::run(tls_server) });
    let plain_server = thread::spawn(move || { hyper::rt::run(server); });

    kafka_sending_thread.join().unwrap();
    kafka_tls_sending_thread.join().unwrap();
    kafka_receiving_thread.join().unwrap();
    plain_server.join().unwrap_err();
    tls_server.join().unwrap_err();
}


// Load public certificate from file.
fn load_certs(filename: &str) -> io::Result<Vec<rustls::Certificate>> {
    // Open certificate file.
    let certfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    let certs = pemfile::certs(&mut reader).map_err(|_| error("failed to load certificate".into())).unwrap();
    info!("Certs = {:?}", certs.len());
    if certs.len() == 0 {
        return Err(error("expected at least one certificate".into()));
    }
    Ok(certs)

}

// Load private key from file.
fn load_private_key(filename: &str) -> io::Result<rustls::PrivateKey> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(keyfile);


    // Load and return a single private key.
    let keys = pemfile::rsa_private_keys(&mut reader);

    let keys = match keys {
        Ok(keys) => keys,
        Err(error) => {
            panic!("There was a problem with reading private key: {:?}", error)
        },
    };
    info!("Keys = {:?}", keys.len());
    if keys.len() != 1 {
        return Err(error("expected a single private key".into()));
    }
    Ok(keys[0].clone())
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

