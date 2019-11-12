extern crate rdkafka;
extern crate rdkafka_sys;

use std::borrow::Borrow;
use std::thread;
use std::thread::JoinHandle;

use crossbeam_channel::Receiver;
use futures::future;
use http::HeaderValue;
use hyper::{Body, Client, Method, Request, Uri};
use hyper::body::Payload;
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;
use hyper::rt::{Future, Stream};
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance, StreamConsumer};
use rdkafka::error::KafkaResult;
use rdkafka::message::{Message, ToBytes};
use rdkafka::producer::{FutureProducer, FutureRecord};
use sled::{Db, IVec};

//use super::utils::structs::{CustomContext, Job, PublishRequest,InternalMessage,KafkaResult,CustomContext};
use super::utils::structs;
use super::utils::structs::*;

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


pub fn configure_broker(kafka_broker_address: String, kafka_sending_topic: String, kafka_receiving_topic: String, kafka_receiving_group: String, db: Db, rx: Receiver<InternalMessage>) -> Vec<JoinHandle<()>> {
    let context = CustomContext;

    let plain_producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_broker_address)
        .set("message.timeout.ms", "5000")
        .create().expect("Can't start broker");


//    let tls_producer:FutureProducer =
//        ClientConfig::new()
//            .set("bootstrap.servers", &kafka_broker_address)
//            .set("message.timeout.ms", "5000")
//            .create().expect("Can't start broker");;


    let consumer: StreamConsumer<CustomContext> = ClientConfig::new()
        .set("group.id", &kafka_receiving_group)
        .set("bootstrap.servers", &kafka_broker_address)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context).expect("Can't start broker");

    let mut topics: Vec<&str> = vec!();
    topics.push(&kafka_receiving_topic);
    let tt = topics.borrow();
    consumer.subscribe(tt).expect("Can't subscribe to topics");

    let client: Client<HttpConnector<GaiResolver>, Body> = Client::new();


    let mut threads: Vec<JoinHandle<()>> = vec!();

    threads.push(create_sending_thread(kafka_receiving_topic, kafka_sending_topic, db.clone(), rx, plain_producer));

    threads.push(create_receiving_thread(consumer, client, db));
    threads
}


fn create_sending_thread(receiving_topic: String, sending_topic: String, db: Db, rx: Receiver<InternalMessage>, producer: FutureProducer) -> JoinHandle<()> {
    let res = thread::spawn(move || {
//        let rx = rx.clone();
//        let sending_topic = sending_topic.clone();
//        let receiving_topic = receiving_topic.clone();
//        let db = db.clone();
//        let producer = producer.clone();
        loop {
            kafka_event_handler(&rx, &producer, &sending_topic, &receiving_topic, &db);
        }
    });
    res
}

fn create_receiving_thread(consumer: StreamConsumer<CustomContext>, client: Client<HttpConnector<GaiResolver>, Body>, db: Db) -> JoinHandle<()> {
    let res = thread::spawn(move || {
        let f = {
            kafka_incoming_event_handler(&consumer, &client, &db);
            future::ok(())
        };
        hyper::rt::run(f);
    });
    res
}





pub fn kafka_event_handler(rx: &Receiver<InternalMessage>, kafka_producer: &FutureProducer, publish_topic: &str, subscribe_topic: &str, db: &Db) {
    let job = rx.recv().unwrap();

    let sender = job.sender;
    match job.job {
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
            let r = FutureRecord::to(publish_topic).payload(ToBytes::to_bytes(&req.payload))
                .key("some key".to_bytes());
            let foo = kafka_producer.send(r, 0).map(move |status| {
                match status {
                    Ok(_) => {
                        sender.send(structs::KafkaResult { result: "All is good".to_string() })
                    }
                    Err(e) => {
                        sender.send(structs::KafkaResult { result: e.0.to_string() })
                    }
                }
            });
            if let Err(_) = foo.wait() {
                warn!("hmmm something is very wrong here. it seems that the channel has been closed");
            }
        }
    }
}

pub fn kafka_incoming_event_handler(consumer: &StreamConsumer<structs::CustomContext>, client: &Client<HttpConnector, Body>, db: &Db) {
    let stream = consumer.start();
    for message in stream.wait() {
        let f = {
            match message {
                Err(_) => {
                    warn!("Error while reading from stream.");
                }
                Ok(Ok(m)) => {
                    let payload = match m.payload_view::<str>() {
                        None => "",
                        Some(Ok(s)) => s,
                        Some(Err(e)) => {
                            warn!("Error while deserializing message payload: {:?}", e);
                            ""
                        }
                    };

                    let mut uri: String = String::from("");
                    if let Ok(maybe_url) = db.get(m.topic()) {
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
                        if f == 200 {
                            consumer.commit_message(&m, CommitMode::Async).unwrap();
                        }
                    }).wait();
                    if let Err(e) = result {
                        warn!("Kafka error when commiting messages: {}", e);
                    }
                }
                Ok(Err(e)) => {
                    warn!("Kafka error: {}", e);
                }
            }
            future::ok(())
        };
        info!("got message. about to spawn");
        hyper::rt::run(f);
    };
}


