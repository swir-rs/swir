use std::borrow::Borrow;
use std::thread;
use std::thread::JoinHandle;

use crossbeam_channel::{bounded, Receiver, Sender};
use futures::future;
use hyper::rt::{Future, Stream};
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{Consumer, ConsumerContext, Rebalance, StreamConsumer};
use rdkafka::error::KafkaResult;
use rdkafka::message::{Message, ToBytes};
use rdkafka::producer::{FutureProducer, FutureRecord};
use sled::{Db, IVec};

use crate::utils;
use super::super::utils::structs;
use super::super::utils::structs::*;

pub type Result<T> = std::result::Result<T, String>;

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


pub fn configure_broker(broker_address: String, sending_topic: String, receiving_topic: String, receiving_group: String, db: Db, rx: Receiver<RestToMessagingContext>, tx: Sender<utils::structs::MessagingToRestContext>) -> Result<Vec<JoinHandle<()>>> {
    let context = CustomContext;

    let plain_producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &broker_address)
        .set("message.timeout.ms", "5000")
        .create().expect("Can't start broker");

    let consumer: StreamConsumer<CustomContext> = ClientConfig::new()
        .set("group.id", &receiving_group)
        .set("bootstrap.servers", &broker_address)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context).expect("Can't start broker");

    let mut topics: Vec<&str> = vec!();
    topics.push(&receiving_topic);
    let tt = topics.borrow();
    consumer.subscribe(tt).expect("Can't subscribe to topics");

    let mut threads: Vec<JoinHandle<()>> = vec!();
    threads.push(create_sending_thread(receiving_topic, sending_topic, db.clone(), rx, plain_producer));
    threads.push(create_receiving_thread(consumer, tx, db));
    Ok(threads)
}


fn create_sending_thread(receiving_topic: String, sending_topic: String, db: Db, rx: Receiver<RestToMessagingContext>, producer: FutureProducer) -> JoinHandle<()> {
    let res = thread::spawn(move || {
        loop {
            kafka_event_handler(&rx, &producer, &sending_topic, &receiving_topic, &db);
        }
    });
    res
}


fn create_receiving_thread(consumer: StreamConsumer<CustomContext>, tx: Sender<utils::structs::MessagingToRestContext>, db: Db) -> JoinHandle<()> {
    let res = thread::spawn(move || { hyper::rt::run(future::lazy(move || { kafka_incoming_event_handler(&consumer, tx, &db) })) });
    res
}


pub fn kafka_event_handler(rx: &Receiver<RestToMessagingContext>, kafka_producer: &FutureProducer, publish_topic: &str, subscribe_topic: &str, db: &Db) {
    let job = rx.recv().unwrap();

    let sender = job.sender;
    match job.job {
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
            info!("Kafka plain sending {:?}", req);
            let r = FutureRecord::to(publish_topic).payload(ToBytes::to_bytes(&req.payload))
                .key("some key".to_bytes());
            let foo = kafka_producer.send(r, 0).map(move |status| {
                match status {
                    Ok(_) => {
                        sender.send(structs::MessagingResult { status: 1, result: "KAFKA is good".to_string() })
                    }
                    Err(e) => {
                        sender.send(structs::MessagingResult { status: 1, result: e.0.to_string() })
                    }
                }
            });
            if let Err(_) = foo.wait() {
                warn!("hmmm something is very wrong here. it seems that the channel has been closed");
            }
        }
    }
}

fn send_request(m: impl Message, tx: &Sender<utils::structs::MessagingToRestContext>, db: &Db) {
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

    let (s, r) = bounded(1);
    let p = MessagingToRestContext {
        sender: s,
        payload: payload.as_bytes().to_vec(),
        uri: uri,
    };

    if let Err(e) = tx.send(p) {
        warn!("Error from the client {}", e)
    }
    match r.recv() {
        Ok(r) => info!("Response from the client {:?}", r),
        Err(e) => warn!("Internal communication error  {}", e)
    }
}

pub fn kafka_incoming_event_handler(consumer: &StreamConsumer<structs::CustomContext>, tx: Sender<utils::structs::MessagingToRestContext>, db: &Db) -> impl Future<Item=(), Error=()> {
    let stream = consumer.start();
    for message in stream.wait() {
        let f = {
            match message {
                Err(e) => {
                    warn!("Error while reading from stream. {:?}", e);
                }
                Ok(Err(e)) => {
                    warn!("Error while reading from stream. {:?} ", e);
                }
                Ok(Ok(m)) => {
                    send_request(m, &tx, &db);
                }
            }
            future::ok(())
        };
        hyper::rt::spawn(f);
    };
    futures::future::ok(())
}


