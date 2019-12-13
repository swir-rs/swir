use std::borrow::Borrow;

use futures::future::FutureExt;
use futures::stream::StreamExt;
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::error::KafkaResult;
use rdkafka::message::{Headers, Message};
use rdkafka::producer::{FutureProducer, FutureRecord};
use sled::{Db, IVec};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::utils;

use super::super::utils::structs;
use super::super::utils::structs::*;

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
        debug!("Committing offsets: {:?}", result);
    }
}

pub async fn configure_broker(
    broker_address: Vec<String>,
    producer_topics: Vec<String>,
    consumer_topics: Vec<String>,
    consumer_groups: Vec<String>,
    db: Db,
    rx: Receiver<RestToMessagingContext>,
    tx: Sender<utils::structs::MessagingToRestContext>,
) {
    info!("Kafka ");

    let f1 = async {
        kafka_incoming_event_handler(
            broker_address.clone(),
            consumer_topics.clone(),
            consumer_groups,
            tx,
            db.clone(),
        )
            .await
    };
    let f2 = async {
        kafka_event_handler(
            rx,
            broker_address.clone(),
            producer_topics,
            consumer_topics.clone(),
            db.clone(),
        )
            .await
    };
    let (_r1, _r2) = futures::join!(f1, f2);
}

pub async fn kafka_event_handler(
    mut rx: Receiver<RestToMessagingContext>,
    broker_address: Vec<String>,
    producer_topics: Vec<String>,
    subscribe_topics: Vec<String>,
    db: Db,
) {
    let kafka_producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", broker_address.get(0).unwrap())
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Can't start broker");

    info!("Kafka running");
    let subscribe_topic = subscribe_topics.get(0).unwrap();
    let producer_topic = producer_topics.get(0).unwrap();

    while let Some(job) = rx.next().await {
        let sender = job.sender;
        match job.job {
            Job::Subscribe(value) => {
                let req = value;
                info!("New registration  {:?}", req);
                if let Err(e) = db.insert(
                    subscribe_topic.clone(),
                    IVec::from(req.endpoint.url.as_bytes()),
                ) {
                    warn!("Can't store registration {:?}", e);
                    if let Err(e) = sender.send(structs::MessagingResult {
                        status: 1,
                        result: e.to_string(),
                    }) {
                        warn!("Can't send response back {:?}", e);
                    }
                } else {
                    if let Err(e) = sender.send(structs::MessagingResult {
                        status: 1,
                        result: "All is good".to_string(),
                    }) {
                        warn!("Can't send response back {:?}", e);
                    }
                };
            }

            Job::Publish(value) => {
                let req = value;
                debug!("Kafka plain sending {:?}", req);
                let r = FutureRecord::to(producer_topic.as_str())
                    .payload(&req.payload)
                    .key("some key");
                let foo = kafka_producer.send(r, 0).map(move |status| match status {
                    Ok(_) => sender.send(structs::MessagingResult {
                        status: 1,
                        result: "KAFKA is good".to_string(),
                    }),
                    Err(e) => sender.send(structs::MessagingResult {
                        status: 1,
                        result: e.to_string(),
                    }),
                });

                if let Err(_) = foo.await {
                    warn!("hmmm something is very wrong here. it seems that the channel has been closed");
                }
            }
        }
    }
}

fn send_request(
    p: Vec<u8>,
    topic: &str,
    mut tx: Sender<utils::structs::MessagingToRestContext>,
    db: &Db,
) {
    let mut uri: String = String::from("");

    debug!("Processing message  {:?}", p);
    if let Ok(maybe_url) = db.get(topic) {
        if let Some(url) = maybe_url {
            let vec = url.to_vec();
            uri = String::from_utf8_lossy(vec.borrow()).to_string();
        }
    }

    let (s, _r) = futures::channel::oneshot::channel();
    let p = MessagingToRestContext {
        sender: s,
        payload: p.to_vec(),
        uri,
    };

    if let Err(e) = tx.try_send(p) {
        warn!("Error from the client {}", e)
    }
    //    match r.recv() {
    //        Ok(r) => debug!("Response from the client {:?}", r),
    //        Err(e) => warn!("Internal communication error  {}", e)
    //    }
}

type LoggingConsumer = StreamConsumer<CustomContext>;

pub async fn kafka_incoming_event_handler(
    broker_address: Vec<String>,
    consumer_topics: Vec<String>,
    consumer_groups: Vec<String>,
    tx: Sender<utils::structs::MessagingToRestContext>,
    db: Db,
) {
    let context = CustomContext;
    let consumer_group = consumer_groups.get(0).unwrap();
    let broker_address = broker_address.get(0).unwrap();

    let consumer: LoggingConsumer = ClientConfig::new()
        .set("group.id", consumer_group)
        .set("bootstrap.servers", broker_address)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed");


    let topics: Vec<&str> = consumer_topics.iter().map(|x| &**x).collect();
    consumer.subscribe(&topics).expect("Can't subscribe to topics");


    let mut message_stream = consumer.start();

    while let Some(message) = message_stream.next().await {
        match message {
            Err(e) => warn!("Kafka error: {}", e),
            Ok(m) => {
                let payload = match m.payload_view::<str>() {
                    None => "",
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        warn!("Error while deserializing message payload: {:?}", e);
                        ""
                    }
                };
                info!("key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                      m.key(), payload, m.topic(), m.partition(), m.offset(), m.timestamp());
                if let Some(headers) = m.headers() {
                    for i in 0..headers.count() {
                        let header = headers.get(i).unwrap();
                        info!("  Header {:#?}: {:?}", header.0, header.1);
                    }
                }
                let t = m.topic().clone();
                let mut vec = Vec::new();
                vec.extend(payload.bytes());
                send_request(vec, t, tx.clone(), &db);
                consumer.commit_message(&m, CommitMode::Async).unwrap();
            }
        };
    }
}
