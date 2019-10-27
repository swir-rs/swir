#![deny(warnings)]

#[macro_use]
extern crate clap;
extern crate futures;
extern crate hyper;
extern crate kafka;

use std::{sync::Arc, time::Duration};
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

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;


fn echo(req: Request<Body>, counter: &AtomicUsize , s:&Sender<usize>) -> BoxFut {
    let mut response = Response::new(Body::empty());
    counter.fetch_add(1, Ordering::Relaxed);
    println!("Counter {}", counter.load(Ordering::Relaxed));
    let headers= req.headers();
    println!("Headers {:?}", headers);
    let (parts,body) = req.into_parts();
    println!("Parts {:?}", parts);
    println!("Body {:?}", body);

    match (parts.method, parts.uri.path()) {
        // Serve some instructions at /
        (Method::GET, "/") => {
            *response.body_mut() = Body::from("Try POSTing data to /echo");
        }

        // Simply echo the body back to the client.
        (Method::POST, "/echo") => {
//            let res = body.concat2().wait().unwrap();
//            println!("{:?}", res);
//            producer.send(&Record::from_value("Request", body.as_bytes())).unwrap();
            println!("Sending {}", counter.load(Ordering::Relaxed));
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
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    println!("Application arguments {:?}", matches);

    let external_address = matches.value_of("address").unwrap().parse().expect("Unable to parse socket address");
    let kafka_sending_topic = matches.value_of("kafka_sending_topic").unwrap().to_owned();
    let kafka_broker_address = matches.value_of("kafka_broker").unwrap();
    let kafka_receiving_topic = matches.value_of("kafka_receiving_topic").unwrap();
    println!("Using kafka broker on {}", kafka_broker_address);
    let kafka_broker_address = vec!(String::from(kafka_broker_address));
    println!("Listening on http://{}", external_address);

    let request_counter = Arc::new(AtomicUsize::new(0));

    let (tx, rx):(Sender<usize>, Receiver<usize>) = mpsc::channel();

    let mut producer =
        Producer::from_hosts(kafka_broker_address.to_owned())
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();

    let mut consumer =
        Consumer::from_hosts(kafka_broker_address.to_owned())
            .with_topic(kafka_receiving_topic.to_owned())
            .with_fallback_offset(FetchOffset::Latest)
//            .with_group(kafka_receiving_group.to_owned())
            .with_offset_storage(GroupOffsetStorage::Kafka)
            .create()
            .unwrap();


    let server = Server::bind(&external_address)
        .serve(move || {
            let inner_rc = Arc::clone(&request_counter);
            let inner_txx = mpsc::Sender::clone(&tx);
            service_fn(move |req| echo(req, &inner_rc,&mpsc::Sender::clone(&inner_txx)))
        }
        ).map_err(|e| eprintln!("server error: {}", e));


    let kafka_sending_thread = thread::spawn(move || {
        let topic = kafka_sending_topic.clone();
        loop {
            let i = rx.recv().unwrap();
            println!("Sending {}", i);
            producer.send(&Record::from_value(&topic, i.to_string())).unwrap();
        }
    });

    let kafka_receiving_thread = thread::spawn(move || {
        loop {
            for ms in consumer.poll().unwrap().iter() {
                for m in ms.messages() {
                    println!("Received message {:?}", std::str::from_utf8(m.value).unwrap());
                }
                let r = consumer.consume_messageset(ms);
                println!("Consumed result {:?}", r);
                consumer.commit_consumed().unwrap();
            }

        }
    });

    hyper::rt::run(server);
    kafka_sending_thread.join().unwrap();
    kafka_receiving_thread.join().unwrap();

}


